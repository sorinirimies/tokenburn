use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::features::usage::{Row, Tool};
use crate::utils::files::{env_path, modified_since, num, num_any, walk_ext};
use crate::utils::time::parse_ts;

/// `$AMP_DATA_DIR` or `~/.local/share/amp` (plus the platform data dir).
pub fn roots() -> Vec<PathBuf> {
    if let Some(o) = env_path("AMP_DATA_DIR") {
        return vec![o];
    }
    let mut out = Vec::new();
    for p in [
        dirs::home_dir().map(|h| h.join(".local/share/amp")),
        dirs::data_local_dir().map(|d| d.join("amp")),
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

pub fn collect_amp(start: DateTime<Utc>) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    for root in roots().into_iter().filter(|r| r.is_dir()) {
        rows.extend(collect_amp_from(&root, start)?);
    }
    Ok(rows)
}

/// Read every `threads/*.json` below `root`.
pub fn collect_amp_from(root: &Path, start: DateTime<Utc>) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    for file in walk_ext(&root.join("threads"), &["json"]) {
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
        rows.extend(parse_thread(&text, &id));
    }
    rows.retain(|r| r.ts >= start);
    Ok(rows)
}

/// Current schema: `messages[].usage`. Legacy: `usageLedger.events[]`.
fn parse_thread(text: &str, id: &str) -> Vec<Row> {
    let Ok(doc) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let from_messages: Vec<Row> = doc
        .get("messages")
        .and_then(Value::as_array)
        .map(|ms| {
            ms.iter()
                .filter_map(|m| usage_row(m.get("usage")?, id))
                .collect()
        })
        .unwrap_or_default();
    if !from_messages.is_empty() {
        return from_messages;
    }
    doc.pointer("/usageLedger/events")
        .and_then(Value::as_array)
        .map(|evs| evs.iter().filter_map(|e| ledger_row(e, id)).collect())
        .unwrap_or_default()
}

fn usage_row(u: &Value, id: &str) -> Option<Row> {
    let input = num(u.get("inputTokens"));
    let output = num(u.get("outputTokens"));
    let cache_write = num(u.get("cacheCreationInputTokens"));
    let cache_read = num(u.get("cacheReadInputTokens"));
    if input + output + cache_write + cache_read == 0 {
        return None;
    }
    Some(Row {
        tool: Tool::Amp,
        project: u
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        id: id.to_string(),
        ts: parse_ts(u.get("timestamp")?.as_str()?)?,
        input,
        output,
        cache_read,
        cache_write,
        cost: 0.0,
    })
}

fn ledger_row(e: &Value, id: &str) -> Option<Row> {
    let t = e.get("tokens")?;
    let input = num_any(t, &["input"]);
    let output = num_any(t, &["output"]);
    if input + output == 0 {
        return None;
    }
    Some(Row {
        tool: Tool::Amp,
        project: e
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        id: id.to_string(),
        ts: parse_ts(e.get("timestamp")?.as_str()?)?,
        input,
        output,
        cache_read: 0,
        cache_write: 0,
        cost: 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_schema_reads_message_usage() {
        let t = r#"{"messages":[
          {"role":"user"},
          {"role":"assistant","usage":{"model":"m","timestamp":"2026-05-01T10:00:00Z",
            "inputTokens":10,"outputTokens":4,"cacheCreationInputTokens":3,"cacheReadInputTokens":7,"totalTokens":24}}]}"#;
        let r = parse_thread(t, "T-1");
        assert_eq!(r.len(), 1);
        assert_eq!(
            (r[0].input, r[0].output, r[0].cache_write, r[0].cache_read),
            (10, 4, 3, 7)
        );
        assert_eq!(r[0].tool, Tool::Amp);
    }

    #[test]
    fn legacy_ledger_is_used_only_without_message_usage() {
        let t = r#"{"usageLedger":{"events":[
          {"timestamp":"2026-05-01T10:00:00Z","model":"m","tokens":{"input":5,"output":2,"total":7}}]}}"#;
        let r = parse_thread(t, "T-2");
        assert_eq!((r.len(), r[0].input, r[0].output), (1, 5, 2));
    }

    #[test]
    fn garbage_and_empty_threads_yield_nothing() {
        assert!(parse_thread("{not json", "x").is_empty());
        assert!(parse_thread("{}", "x").is_empty());
    }

    #[test]
    fn reads_a_threads_directory() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("threads")).unwrap();
        std::fs::write(
            d.path().join("threads/T-9.json"),
            r#"{"messages":[{"usage":{"model":"m","timestamp":"2026-05-01T10:00:00Z","inputTokens":1,"outputTokens":1}}]}"#,
        )
        .unwrap();
        let r = collect_amp_from(d.path(), DateTime::<Utc>::UNIX_EPOCH).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "T-9");
    }
}
