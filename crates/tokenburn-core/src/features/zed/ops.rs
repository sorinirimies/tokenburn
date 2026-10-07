use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use serde_json::Value;

use crate::features::usage::{Row, Tool};
use crate::utils::files::num_f;
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

/// Decoded threads, keyed by `(database path, thread id)` and valid while the
/// thread's `updated_at` is unchanged.
///
/// Decoding a thread means zstd-decompressing and parsing a JSON blob that can
/// be megabytes, and Zed rewrites the database whenever a conversation moves —
/// so a per-file signature would re-decode *every* thread on each refresh. This
/// way only the threads that actually changed are decoded again.
type ThreadKey = (PathBuf, String);
/// `updated_at` stamp the row was decoded at, and the row (`None` = no usage).
type ThreadEntry = (String, Option<Row>);
type ThreadMap = HashMap<ThreadKey, ThreadEntry>;
static THREADS: LazyLock<Mutex<ThreadMap>> = LazyLock::new(|| Mutex::new(HashMap::new()));

fn threads() -> std::sync::MutexGuard<'static, ThreadMap> {
    THREADS.lock().unwrap_or_else(|e| e.into_inner())
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

    // 1. Cheap metadata only — no blobs.
    let mut stmt = conn
        .prepare("SELECT id, created_at, updated_at, data_type FROM threads WHERE created_at >= ?1")
        .context("preparing threads query")?;
    let metas: Vec<(String, String, String, Option<String>)> = stmt
        .query_map([start_iso], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                r.get::<_, Option<String>>(3)?,
            ))
        })?
        .flatten()
        .collect();

    // 2. Decode only threads that are new or whose `updated_at` moved.
    let mut blob = conn
        .prepare("SELECT data FROM threads WHERE id = ?1")
        .context("preparing thread blob query")?;
    let mut rows = Vec::new();
    let mut live: Vec<ThreadKey> = Vec::with_capacity(metas.len());
    for (id, created_at, updated_at, data_type) in metas {
        let key = (db.to_path_buf(), id.clone());
        live.push(key.clone());
        let cached = threads()
            .get(&key)
            .filter(|(u, _)| *u == updated_at)
            .map(|(_, row)| row.clone());
        let row = match cached {
            Some(row) => row,
            None => {
                let decoded = parse_ts(&created_at).and_then(|ts| {
                    let data: Vec<u8> = blob.query_row([&id], |r| r.get(0)).ok()?;
                    decode_thread(id, ts, data_type.as_deref(), &data)
                });
                threads().insert(key, (updated_at, decoded.clone()));
                decoded
            }
        };
        rows.extend(row);
    }

    // Forget threads that left this database (deleted) or fell out of the window.
    let live: std::collections::HashSet<ThreadKey> = live.into_iter().collect();
    threads().retain(|k, _| k.0 != db || live.contains(k));
    Ok(rows)
}

#[cfg(test)]
pub(crate) fn cached_threads(db: &Path) -> usize {
    threads().keys().filter(|k| k.0 == db).count()
}

/// The two things tokenburn needs from a Zed thread document; the (large)
/// conversation itself is skipped while parsing, never allocated.
#[derive(Deserialize)]
struct Thread {
    model: Option<Value>,
    cumulative_token_usage: Option<ThreadUsage>,
}

#[derive(Deserialize)]
struct ThreadUsage {
    input_tokens: Option<f64>,
    output_tokens: Option<f64>,
}

/// Compressed blobs larger than this are decompressed as a stream instead of in one
/// go. A long conversation can be tens of megabytes compressed and many times that as
/// JSON, and holding all of it at once was the biggest memory peak of a cold start;
/// the stream keeps it to a few megabytes. Streaming is slower, which is fine for the
/// few huge threads (and a thread is only ever decoded again once it changes).
const STREAM_ABOVE: usize = 4 * 1024 * 1024;

/// Decode one thread blob into a [`Row`].
///
/// Small blobs are decompressed in one go and parsed *typed* (`from_slice`, which is
/// markedly faster than `from_reader`): only the two fields above are kept and the
/// conversation is skipped without allocating. Huge ones are streamed (see
/// [`STREAM_ABOVE`]). One thread at a time is ever in memory.
pub(crate) fn decode_thread(
    id: String,
    ts: DateTime<Utc>,
    data_type: Option<&str>,
    data: &[u8],
) -> Option<Row> {
    decode_thread_with(id, ts, data_type, data, STREAM_ABOVE)
}

fn decode_thread_with(
    id: String,
    ts: DateTime<Utc>,
    data_type: Option<&str>,
    data: &[u8],
    stream_above: usize,
) -> Option<Row> {
    let thread: Thread = if data_type == Some("zstd") {
        if data.len() > stream_above {
            let decoder = zstd::Decoder::new(data).ok()?;
            serde_json::from_reader(std::io::BufReader::with_capacity(64 * 1024, decoder)).ok()?
        } else {
            serde_json::from_slice(&zstd::decode_all(data).ok()?).ok()?
        }
    } else {
        serde_json::from_slice(data).ok()?
    };
    let usage = thread.cumulative_token_usage;
    let model = thread
        .model
        .as_ref()
        .and_then(|m| m.get("model"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    Some(Row {
        tool: Tool::Zed,
        project: model,
        id,
        ts,
        input: num_f(usage.as_ref().and_then(|u| u.input_tokens)),
        output: num_f(usage.as_ref().and_then(|u| u.output_tokens)),
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
    fn only_changed_threads_are_decoded_again() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("threads.db");
        let conn = Connection::open(&db).unwrap();
        conn.execute(
            "CREATE TABLE threads (id TEXT PRIMARY KEY, created_at TEXT, updated_at TEXT, data_type TEXT, data BLOB)",
            [],
        )
        .unwrap();
        let ins = |id: &str, updated: &str, json: &str| {
            conn.execute(
                "INSERT OR REPLACE INTO threads VALUES (?1, '2026-05-01T10:00:00Z', ?2, 'json', ?3)",
                rusqlite::params![id, updated, json.as_bytes()],
            )
            .unwrap();
        };
        ins("t1", "u1", JSON);
        ins("t2", "u1", JSON);
        let start = "2026-01-01T00:00:00Z".parse().unwrap();
        assert_eq!(collect_zed_from(&db, start).unwrap().len(), 2);
        assert_eq!(cached_threads(&db), 2);

        // Corrupt t1's blob WITHOUT bumping updated_at: a cached thread is not re-read, so
        // the stale (good) row must still be served; t2 is genuinely updated and re-decoded.
        ins("t1", "u1", "not json");
        ins(
            "t2",
            "u2",
            r#"{"cumulative_token_usage":{"input_tokens":100,"output_tokens":1}}"#,
        );
        let rows = collect_zed_from(&db, start).unwrap();
        assert_eq!(rows.len(), 2);
        let t1 = rows.iter().find(|r| r.id == "t1").unwrap();
        let t2 = rows.iter().find(|r| r.id == "t2").unwrap();
        assert_eq!(t1.input, 7, "t1 came from the cache");
        assert_eq!(
            t2.input, 100,
            "t2 was decoded again after its updated_at changed"
        );

        // Deleting a thread drops it from the results and from the cache.
        conn.execute("DELETE FROM threads WHERE id = 't1'", [])
            .unwrap();
        assert_eq!(collect_zed_from(&db, start).unwrap().len(), 1);
        assert_eq!(cached_threads(&db), 1);
    }

    #[test]
    fn reads_sqlite() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("threads.db");
        let conn = Connection::open(&db).unwrap();
        conn.execute(
            "CREATE TABLE threads (id TEXT, created_at TEXT, updated_at TEXT, data_type TEXT, data BLOB)",
            [],
        )
        .unwrap();
        let packed = zstd::encode_all(JSON.as_bytes(), 0).unwrap();
        conn.execute(
            "INSERT INTO threads VALUES ('t1', '2026-05-01T10:00:00Z', 'u', 'zstd', ?1)",
            [packed],
        )
        .unwrap();
        drop(conn);

        let start = "2026-01-01T00:00:00Z".parse().unwrap();
        let rows = collect_zed_from(&db, start).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "t1");
    }

    #[test]
    fn huge_blobs_are_streamed_and_decode_to_exactly_the_same_row() {
        // A thread whose conversation is large (incompressible-ish text), compressed.
        let mut filler = String::new();
        let mut x: u64 = 0x2545_f491_4f6c_dd1d;
        for _ in 0..800_000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            filler.push_str(&format!("{x:016x}"));
        }
        let json = format!(
            r#"{{"model":{{"model":"claude-big"}},"messages":["{filler}"],"cumulative_token_usage":{{"input_tokens":1234,"output_tokens":56}}}}"#
        );
        let blob = zstd::encode_all(json.as_bytes(), 1).unwrap();
        assert!(
            blob.len() > STREAM_ABOVE,
            "the fixture must exceed the streaming threshold ({} bytes)",
            blob.len()
        );

        let ts: DateTime<Utc> = "2026-05-01T10:00:00Z".parse().unwrap();
        let streamed = decode_thread("t".into(), ts, Some("zstd"), &blob).unwrap();
        // Force the one-shot path on the same bytes: the results must agree.
        let whole = decode_thread_with("t".into(), ts, Some("zstd"), &blob, usize::MAX).unwrap();
        assert_eq!((streamed.input, streamed.output), (1234, 56));
        assert_eq!(streamed.project, "claude-big");
        assert_eq!(
            (streamed.input, streamed.output, &streamed.project),
            (whole.input, whole.output, &whole.project)
        );
    }
}
