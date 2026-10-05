use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::features::usage::{Row, Tool};
use crate::utils::time::parse_ts;

/// `~/.pi/agent/sessions`, if a home directory can be determined.
pub fn sessions_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".pi").join("agent").join("sessions"))
}

/// Collect pi usage rows from the default sessions directory.
///
/// A missing directory is not an error — it simply yields no rows.
pub fn collect_pi(start: DateTime<Utc>) -> Result<Vec<Row>> {
    match sessions_dir() {
        Some(dir) if dir.is_dir() => collect_pi_from(&dir, start),
        _ => Ok(Vec::new()),
    }
}

/// Collect pi usage rows from every `*.jsonl` below `root`.
///
/// Files not modified since `start` are skipped as a cheap pre-filter; exact
/// per-turn filtering by timestamp is left to the caller.
pub fn collect_pi_from(root: &Path, start: DateTime<Utc>) -> Result<Vec<Row>> {
    let mut files = Vec::new();
    walk_jsonl(root, &mut files).with_context(|| format!("reading {}", root.display()))?;

    let start_sys: SystemTime = start.into();
    let mut rows = Vec::new();
    for file in files {
        let modified = fs::metadata(&file).and_then(|m| m.modified());
        if matches!(modified, Ok(m) if m < start_sys) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&file) else {
            tracing::warn!("pi: cannot read {}", file.display());
            continue;
        };
        let project = file
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().trim_matches('-').to_string())
            .unwrap_or_default();
        let id = file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        rows.extend(text.lines().filter_map(|l| parse_line(l, &project, &id)));
    }
    Ok(rows)
}

fn walk_jsonl(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            // Unreadable subdirectories are skipped, not fatal.
            let _ = walk_jsonl(&path, out);
        } else if path.extension().is_some_and(|e| e == "jsonl") {
            out.push(path);
        }
    }
    Ok(())
}

fn num(v: Option<&Value>) -> u64 {
    v.and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
        .unwrap_or(0)
}

/// Parse one jsonl line into a [`Row`]; `None` unless it carries
/// `message.usage` and a top-level `timestamp`.
pub(crate) fn parse_line(line: &str, project: &str, id: &str) -> Option<Row> {
    let rec: Value = serde_json::from_str(line).ok()?;
    let usage = rec.get("message")?.get("usage")?;
    if !usage.is_object() {
        return None;
    }
    let ts = parse_ts(rec.get("timestamp")?.as_str()?)?;
    Some(Row {
        tool: Tool::Pi,
        project: project.to_string(),
        id: id.to_string(),
        ts,
        input: num(usage.get("input")),
        output: num(usage.get("output")),
        cache_read: num(usage.get("cacheRead")),
        cache_write: num(usage.get("cacheWrite")),
        cost: usage
            .get("cost")
            .and_then(|c| c.get("total"))
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = r#"{"timestamp":"2026-05-01T10:00:00Z","message":{"usage":{"input":10,"output":5,"cacheRead":3,"cacheWrite":2,"cost":{"total":0.25}}}}"#;

    #[test]
    fn parses_usage_line() {
        let r = parse_line(LINE, "proj", "a.jsonl").unwrap();
        assert_eq!(
            (r.input, r.output, r.cache_read, r.cache_write),
            (10, 5, 3, 2)
        );
        assert!((r.cost - 0.25).abs() < 1e-9);
        assert_eq!(r.tool, Tool::Pi);
    }

    #[test]
    fn skips_lines_without_usage_or_timestamp() {
        assert!(parse_line(r#"{"message":{"role":"user"}}"#, "p", "i").is_none());
        assert!(parse_line(r#"{"message":{"usage":{"input":1}}}"#, "p", "i").is_none());
        assert!(parse_line("not json", "p", "i").is_none());
    }

    #[test]
    fn collects_from_directory() {
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("--home-me-proj--");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("s.jsonl"), format!("{LINE}\ngarbage\n{LINE}\n")).unwrap();
        let start = "1970-01-01T00:00:00Z".parse().unwrap();
        let rows = collect_pi_from(dir.path(), start).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].project, "home-me-proj");
    }
}
