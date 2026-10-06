use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::features::usage::{Row, Tool};
use crate::utils::cache::{FileCache, Sig};
use crate::utils::files::{env_path, modified_since, num, read_lines, walk_ext};
use crate::utils::time::parse_ts;

/// `$COPILOT_HOME` (default `~/.copilot`).
pub fn root() -> Option<PathBuf> {
    env_path("COPILOT_HOME").or_else(|| dirs::home_dir().map(|h| h.join(".copilot")))
}

pub fn collect_copilot(start: DateTime<Utc>) -> Result<Vec<Row>> {
    match root().map(|r| r.join("session-state")) {
        Some(dir) if dir.is_dir() => collect_copilot_from(&dir, start),
        _ => Ok(Vec::new()),
    }
}

/// Read every `session-state/<id>.jsonl` and `session-state/<id>/events.jsonl`.
///
/// Copilot CLI records usage in one `session.shutdown` event per session, with
/// per-model counters in `data.modelMetrics`. Sessions that never shut down
/// cleanly, and older CLI versions that log no usage, yield no rows.
/// Parsed rows per session file, reused while the file is unchanged.
static CACHE: LazyLock<FileCache<Vec<Row>>> = LazyLock::new(FileCache::default);

pub fn collect_copilot_from(dir: &Path, start: DateTime<Utc>) -> Result<Vec<Row>> {
    let files = walk_ext(dir, &["jsonl"]);
    let live: HashSet<PathBuf> = files.iter().cloned().collect();
    let mut rows = Vec::new();
    for file in files {
        if !modified_since(&file, start) {
            continue;
        }
        let Some(sig) = Sig::of(&file, 0) else {
            continue;
        };
        let parsed = CACHE.get_or_parse(&file, sig, || parse_file(&file));
        rows.extend(parsed.iter().cloned());
    }
    CACHE.prune_under(dir, &live);
    Ok(rows)
}

fn parse_file(file: &Path) -> Vec<Row> {
    let id = session_id(file);
    let Some(lines) = read_lines(file) else {
        return Vec::new();
    };
    lines
        // Cheap pre-filter: shutdown events are rare, don't JSON-parse the rest.
        .filter(|l| l.contains("session.shutdown"))
        .flat_map(|l| parse_shutdown(&l, &id))
        .collect()
}

/// `…/<uuid>/events.jsonl` → `<uuid>`; `…/<uuid>.jsonl` → `<uuid>`.
fn session_id(file: &Path) -> String {
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if stem == "events" {
        file.parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or(stem)
    } else {
        stem
    }
}

fn parse_shutdown(line: &str, id: &str) -> Vec<Row> {
    let Ok(rec) = serde_json::from_str::<Value>(line) else {
        return Vec::new();
    };
    if rec.get("type").and_then(Value::as_str) != Some("session.shutdown") {
        return Vec::new();
    }
    let Some(ts) = rec
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(parse_ts)
    else {
        return Vec::new();
    };
    let Some(metrics) = rec.pointer("/data/modelMetrics").and_then(Value::as_object) else {
        return Vec::new();
    };
    metrics
        .iter()
        .filter_map(|(model, m)| {
            let u = m.get("usage")?;
            let cache_read = num(u.get("cacheReadTokens"));
            let cache_write = num(u.get("cacheWriteTokens"));
            let output = num(u.get("outputTokens")) + num(u.get("reasoningTokens"));
            // Copilot's inputTokens includes the cached portions: split them out.
            let input = num(u.get("inputTokens")).saturating_sub(cache_read + cache_write);
            if input + output + cache_read + cache_write == 0 {
                return None;
            }
            Some(Row {
                tool: Tool::Copilot,
                project: model.clone(),
                id: id.to_string(),
                ts,
                input,
                output,
                cache_read,
                cache_write,
                cost: 0.0,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn shutdown() -> String {
        json!({"type":"session.shutdown","timestamp":"2026-04-15T09:52:27.352Z","data":{"modelMetrics":{
            "m-a":{"usage":{"inputTokens":100,"outputTokens":50,"cacheReadTokens":10,"cacheWriteTokens":20,"reasoningTokens":5}},
            "m-b":{"usage":{"inputTokens":0,"outputTokens":0}}}}})
        .to_string()
    }

    #[test]
    fn splits_cache_out_of_input_and_adds_reasoning_to_output() {
        let rows = parse_shutdown(&shutdown(), "s");
        assert_eq!(rows.len(), 1, "the all-zero model is dropped");
        let r = &rows[0];
        assert_eq!(
            (r.input, r.output, r.cache_read, r.cache_write),
            (70, 55, 10, 20)
        );
        assert_eq!(r.project, "m-a");
    }

    #[test]
    fn old_logs_without_usage_yield_nothing() {
        // The Copilot CLI logs seen on real machines before usage tracking: only
        // turn/tool events and `session.truncation` (context size, not usage).
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("abc.jsonl"),
            r#"{"type":"session.start","data":{},"timestamp":"2025-12-17T11:38:25.130Z"}
{"type":"session.truncation","data":{"tokenLimit":128000},"timestamp":"2025-12-17T11:39:19.716Z"}
{"type":"assistant.message","data":{"content":"hi"},"timestamp":"2025-12-17T11:39:22.000Z"}
"#,
        )
        .unwrap();
        assert!(collect_copilot_from(d.path(), DateTime::<Utc>::UNIX_EPOCH)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn reads_both_flat_and_nested_session_files() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("flat.jsonl"), shutdown()).unwrap();
        std::fs::create_dir_all(d.path().join("sess-2")).unwrap();
        std::fs::write(d.path().join("sess-2/events.jsonl"), shutdown()).unwrap();
        let mut ids: Vec<_> = collect_copilot_from(d.path(), DateTime::<Utc>::UNIX_EPOCH)
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        ids.sort();
        assert_eq!(ids, ["flat", "sess-2"]);
    }

    #[test]
    fn garbage_is_ignored() {
        assert!(parse_shutdown("session.shutdown but not json", "s").is_empty());
    }
}
