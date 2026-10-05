use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use crate::features::usage::{Row, Tool};
use crate::utils::files::{dir_basename, env_path, modified_since, num, walk_ext};

/// OpenCode data directories: `$OPENCODE_DATA_DIR`, else
/// `$XDG_DATA_HOME/opencode` and `~/.local/share/opencode` (OpenCode uses the XDG
/// layout on every OS), plus the platform data dir (`%LOCALAPPDATA%` on Windows).
pub fn roots() -> Vec<PathBuf> {
    resolve_roots(
        env_path("OPENCODE_DATA_DIR"),
        env_path("XDG_DATA_HOME"),
        dirs::home_dir(),
        dirs::data_local_dir(),
    )
}

pub(crate) fn resolve_roots(
    over: Option<PathBuf>,
    xdg_data: Option<PathBuf>,
    home: Option<PathBuf>,
    data_local: Option<PathBuf>,
) -> Vec<PathBuf> {
    if let Some(o) = over {
        return vec![o];
    }
    let mut out = Vec::new();
    for p in [
        xdg_data.map(|d| d.join("opencode")),
        home.map(|h| h.join(".local/share/opencode")),
        data_local.map(|d| d.join("opencode")),
    ]
    .into_iter()
    .flatten()
    {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

pub fn collect_opencode(start: DateTime<Utc>) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    let mut seen = HashSet::new();
    for root in roots().into_iter().filter(|r| r.is_dir()) {
        rows.extend(collect_dir(&root, start, &mut seen));
    }
    Ok(rows)
}

/// Read one data directory: SQLite databases first (the current format), then the
/// legacy `storage/message/**.json` files, de-duplicated by message id.
pub fn collect_opencode_from(root: &Path, start: DateTime<Utc>) -> Result<Vec<Row>> {
    Ok(collect_dir(root, start, &mut HashSet::new()))
}

fn collect_dir(root: &Path, start: DateTime<Utc>, seen: &mut HashSet<String>) -> Vec<Row> {
    let mut rows = Vec::new();
    for db in databases(root) {
        match read_db(&db, start, seen) {
            Ok(r) => rows.extend(r),
            Err(e) => tracing::warn!("opencode: {}: {e}", db.display()),
        }
    }
    for file in walk_ext(&root.join("storage/message"), &["json"]) {
        if !modified_since(&file, start) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let id = file
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let session = file
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if let Some(row) = parse_message(&text, &session, None).filter(|_| seen.insert(id.clone()))
        {
            rows.push(row);
        }
    }
    rows.retain(|r| r.ts >= start);
    rows
}

/// `opencode.db` and `opencode-*.db`.
fn databases(root: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut dbs: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                n == "opencode.db" || (n.starts_with("opencode-") && n.ends_with(".db"))
            })
        })
        .collect();
    dbs.sort();
    dbs
}

fn read_db(db: &Path, start: DateTime<Utc>, seen: &mut HashSet<String>) -> Result<Vec<Row>> {
    let conn = Connection::open_with_flags(
        db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    let has_message: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='message'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if !has_message {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare(
        "SELECT id, session_id, time_created, data FROM message WHERE time_created >= ?1",
    )?;
    let mapped = stmt.query_map([start.timestamp_millis()], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    let mut rows = Vec::new();
    for item in mapped.flatten() {
        let (id, session, created, data) = item;
        if !seen.insert(id) {
            continue;
        }
        if let Some(row) = parse_message(&data, &session, Some(created)) {
            rows.push(row);
        }
    }
    Ok(rows)
}

/// One assistant message's JSON (`data` column or a legacy message file).
fn parse_message(json: &str, session: &str, created_ms: Option<i64>) -> Option<Row> {
    let v: Value = serde_json::from_str(json).ok()?;
    if v.get("role")
        .and_then(Value::as_str)
        .is_some_and(|r| r != "assistant")
    {
        return None;
    }
    let t = v.get("tokens").filter(|t| t.is_object())?;
    let cache = t.get("cache");
    // OpenCode reports reasoning separately; it is billed as output.
    let output = num(t.get("output")) + num(t.get("reasoning"));
    let input = num(t.get("input"));
    let cache_read = num(cache.and_then(|c| c.get("read")));
    let cache_write = num(cache.and_then(|c| c.get("write")));
    if input + output + cache_read + cache_write == 0 {
        return None;
    }
    let ms = created_ms.or_else(|| v.pointer("/time/created").and_then(Value::as_i64))?;
    let ts = Utc.timestamp_millis_opt(ms).single()?;
    let project = v
        .pointer("/path/cwd")
        .and_then(Value::as_str)
        .and_then(dir_basename)
        .or_else(|| v.get("modelID").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| "unknown".into());
    Some(Row {
        tool: Tool::OpenCode,
        project,
        id: session.to_string(),
        ts,
        input,
        output,
        cache_read,
        cache_write,
        cost: v.get("cost").and_then(Value::as_f64).unwrap_or(0.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MSG: &str = r#"{"role":"assistant","modelID":"claude-x","providerID":"anthropic",
      "time":{"created":1777629600000},"path":{"cwd":"/home/me/app"},"cost":0.25,
      "tokens":{"input":100,"output":20,"reasoning":5,"cache":{"read":40,"write":8}}}"#;

    #[test]
    fn parses_tokens_cost_project_and_time() {
        let r = parse_message(MSG, "ses_1", None).unwrap();
        assert_eq!(
            (r.input, r.output, r.cache_read, r.cache_write),
            (100, 25, 40, 8)
        );
        assert!((r.cost - 0.25).abs() < 1e-9);
        assert_eq!(r.project, "app");
        assert_eq!(r.ts.timestamp_millis(), 1777629600000);
        assert_eq!(r.tool, Tool::OpenCode);
    }

    #[test]
    fn user_messages_and_empty_usage_are_skipped() {
        assert!(parse_message(r#"{"role":"user","tokens":{"input":5}}"#, "s", Some(1)).is_none());
        assert!(parse_message(
            r#"{"role":"assistant","tokens":{"input":0,"output":0}}"#,
            "s",
            Some(1)
        )
        .is_none());
        assert!(parse_message("garbage", "s", Some(1)).is_none());
    }

    fn make_db(dir: &Path) {
        let c = Connection::open(dir.join("opencode.db")).unwrap();
        c.execute(
            "CREATE TABLE message (id TEXT, session_id TEXT, time_created INTEGER, data TEXT)",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO message VALUES ('m1','s1',1777629600000,?1)",
            [MSG],
        )
        .unwrap();
        c.execute("INSERT INTO message VALUES ('m0','s1',1000,?1)", [MSG])
            .unwrap();
    }

    #[test]
    fn reads_the_sqlite_database_and_honours_the_start() {
        let d = tempfile::tempdir().unwrap();
        make_db(d.path());
        let start = Utc.timestamp_millis_opt(1_700_000_000_000).unwrap();
        let rows = collect_opencode_from(d.path(), start).unwrap();
        assert_eq!(rows.len(), 1, "the 1970 message is before `start`");
    }

    #[test]
    fn legacy_json_is_read_and_deduplicated_behind_the_database() {
        let d = tempfile::tempdir().unwrap();
        make_db(d.path());
        let legacy = d.path().join("storage/message/s1");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("m1.json"), MSG).unwrap(); // same id as the db row
        std::fs::write(legacy.join("m2.json"), MSG).unwrap(); // genuinely new
        let rows = collect_opencode_from(d.path(), DateTime::<Utc>::UNIX_EPOCH).unwrap();
        assert_eq!(
            rows.len(),
            3,
            "m1 (db) + m0 (db) + m2 (legacy); the m1 legacy copy is dropped"
        );
    }

    #[test]
    fn a_database_without_a_message_table_is_not_an_error() {
        let d = tempfile::tempdir().unwrap();
        Connection::open(d.path().join("opencode.db"))
            .unwrap()
            .execute("CREATE TABLE other (x)", [])
            .unwrap();
        assert!(collect_opencode_from(d.path(), DateTime::<Utc>::UNIX_EPOCH)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn roots_prefer_the_override_then_xdg_and_home() {
        assert_eq!(
            resolve_roots(Some("/o".into()), None, None, None),
            [PathBuf::from("/o")]
        );
        let r = resolve_roots(
            None,
            Some("/x".into()),
            Some("/h".into()),
            Some("/h/.local/share".into()),
        );
        assert_eq!(
            r,
            [
                PathBuf::from("/x/opencode"),
                PathBuf::from("/h/.local/share/opencode")
            ]
        );
    }
}
