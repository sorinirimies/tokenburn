use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use crate::features::usage::{Row, Tool};
use crate::utils::time::parse_ts;

/// Known `threads.db` locations, most specific first.
///
/// * `TOKENBURN_ZED_DB` — explicit override (a path to `threads.db`)
/// * macOS: `~/Library/Application Support/Zed/threads/threads.db`
/// * Windows: `%LOCALAPPDATA%\Zed\threads\threads.db`
/// * Linux: `$XDG_DATA_HOME/zed/threads/threads.db` (default `~/.local/share`),
///   the Flatpak copy under `~/.var/app/dev.zed.Zed/`, then the legacy
///   `~/.config/zed/threads/threads.db`
///
/// Every platform's candidates are listed on every OS — only ones that exist
/// are used, which also covers e.g. a Windows home mounted under WSL.
pub fn db_candidates() -> Vec<PathBuf> {
    let over = std::env::var_os("TOKENBURN_ZED_DB")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from);
    let mut out: Vec<PathBuf> = over.into_iter().collect();
    out.extend(candidates_from(
        dirs::home_dir().as_deref(),
        dirs::data_local_dir().as_deref(),
    ));
    out
}

/// Platform candidates for a given home and per-user local data directory
/// (`%LOCALAPPDATA%` / `$XDG_DATA_HOME` / `~/Library/Application Support`).
pub(crate) fn candidates_from(home: Option<&Path>, data_local: Option<&Path>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut push = |p: PathBuf| {
        if !out.contains(&p) {
            out.push(p);
        }
    };
    if let Some(d) = data_local {
        // macOS: …/Application Support/Zed · Windows: …\AppData\Local\Zed · Linux: …/zed
        push(d.join("Zed/threads/threads.db"));
        push(d.join("zed/threads/threads.db"));
    }
    if let Some(h) = home {
        push(h.join("Library/Application Support/Zed/threads/threads.db"));
        push(h.join(".local/share/zed/threads/threads.db"));
        push(h.join(".var/app/dev.zed.Zed/data/zed/threads/threads.db"));
        push(h.join(".config/zed/threads/threads.db"));
    }
    out
}

/// Collect Zed rows from the first existing default database.
///
/// No database is not an error — it yields no rows.
pub fn collect_zed(start: DateTime<Utc>) -> Result<Vec<Row>> {
    match db_candidates().into_iter().find(|p| p.is_file()) {
        Some(db) => collect_zed_from(&db, start),
        None => Ok(Vec::new()),
    }
}

/// Collect one row per thread created at or after `start` from `db`.
///
/// Zed stores only `cumulative_token_usage` per thread (no per-message
/// timestamp and no `$` cost), so cost is always `0`.
pub fn collect_zed_from(db: &Path, start: DateTime<Utc>) -> Result<Vec<Row>> {
    let conn = Connection::open_with_flags(
        db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("opening {}", db.display()))?;

    let start_iso = start.format("%Y-%m-%dT%H:%M:%S").to_string();
    let mut stmt = conn
        .prepare("SELECT id, created_at, data_type, data FROM threads WHERE created_at >= ?1")
        .context("preparing threads query")?;

    let mapped = stmt.query_map([start_iso], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Vec<u8>>(3)?,
        ))
    })?;

    let mut rows = Vec::new();
    for item in mapped {
        let Ok((id, created_at, data_type, data)) = item else {
            continue;
        };
        let Some(ts) = parse_ts(&created_at) else {
            continue;
        };
        let Some(row) = decode_thread(id, ts, data_type.as_deref(), &data) else {
            continue;
        };
        rows.push(row);
    }
    Ok(rows)
}

/// Decode one thread blob into a [`Row`].
pub(crate) fn decode_thread(
    id: String,
    ts: DateTime<Utc>,
    data_type: Option<&str>,
    data: &[u8],
) -> Option<Row> {
    let raw = if data_type == Some("zstd") {
        zstd::decode_all(data).ok()?
    } else {
        data.to_vec()
    };
    let obj: Value = serde_json::from_slice(&raw).ok()?;
    let usage = obj.get("cumulative_token_usage");
    let get = |k: &str| {
        usage
            .and_then(|u| u.get(k))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    let model = obj
        .get("model")
        .and_then(|m| m.get("model"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    Some(Row {
        tool: Tool::Zed,
        project: model,
        id,
        ts,
        input: get("input_tokens"),
        output: get("output_tokens"),
        cache_read: 0,
        cache_write: 0,
        cost: 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r#"{"model":{"model":"claude"},"cumulative_token_usage":{"input_tokens":7,"output_tokens":3}}"#;

    #[test]
    fn candidates_cover_every_platform_without_duplicates() {
        let home = Path::new("/home/u");
        let c = candidates_from(Some(home), Some(Path::new("/home/u/.local/share")));
        let s: Vec<String> = c
            .iter()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();
        assert!(s.contains(&"/home/u/.local/share/zed/threads/threads.db".to_string()));
        assert!(
            s.contains(&"/home/u/Library/Application Support/Zed/threads/threads.db".to_string())
        );
        assert!(s.contains(&"/home/u/.var/app/dev.zed.Zed/data/zed/threads/threads.db".to_string()));
        // `data_local/zed` and `~/.local/share/zed` are the same path: listed once.
        let n = s
            .iter()
            .filter(|p| p.ends_with(".local/share/zed/threads/threads.db"))
            .count();
        assert_eq!(n, 1);
    }

    #[test]
    fn windows_style_local_appdata_is_used() {
        let c = candidates_from(None, Some(Path::new("C:/Users/u/AppData/Local")));
        assert_eq!(
            c[0],
            Path::new("C:/Users/u/AppData/Local").join("Zed/threads/threads.db")
        );
    }

    #[test]
    fn decodes_plain_and_zstd() {
        let ts = "2026-05-01T10:00:00Z".parse().unwrap();
        let plain = decode_thread("a".into(), ts, Some("json"), JSON.as_bytes()).unwrap();
        assert_eq!((plain.input, plain.output), (7, 3));
        assert_eq!(plain.project, "claude");

        let packed = zstd::encode_all(JSON.as_bytes(), 0).unwrap();
        let z = decode_thread("b".into(), ts, Some("zstd"), &packed).unwrap();
        assert_eq!((z.input, z.output), (7, 3));
    }

    #[test]
    fn reads_sqlite() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("threads.db");
        let conn = Connection::open(&db).unwrap();
        conn.execute(
            "CREATE TABLE threads (id TEXT, created_at TEXT, data_type TEXT, data BLOB)",
            [],
        )
        .unwrap();
        let packed = zstd::encode_all(JSON.as_bytes(), 0).unwrap();
        conn.execute(
            "INSERT INTO threads VALUES ('t1', '2026-05-01T10:00:00Z', 'zstd', ?1)",
            [packed],
        )
        .unwrap();
        drop(conn);

        let start = "2026-01-01T00:00:00Z".parse().unwrap();
        let rows = collect_zed_from(&db, start).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "t1");
    }
}
