use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::features::usage::{Row, Tool};
use crate::utils::cache::{FileCache, Sig};
use crate::utils::files::{dir_basename, env_path, modified_since, num, read_lines, walk_ext};
use crate::utils::time::parse_ts;

/// `$CODEX_HOME` (default `~/.codex`) → its `sessions/` and `archived_sessions/`.
pub fn roots() -> Vec<PathBuf> {
    let home = env_path("CODEX_HOME").or_else(|| dirs::home_dir().map(|h| h.join(".codex")));
    home.map(|h| vec![h.join("sessions"), h.join("archived_sessions")])
        .unwrap_or_default()
}

pub fn collect_codex(start: DateTime<Utc>) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    for root in roots().into_iter().filter(|r| r.is_dir()) {
        rows.extend(collect_codex_from(&root, start)?);
    }
    Ok(rows)
}

/// Parsed rows per rollout file, reused while the file is unchanged.
static CACHE: LazyLock<FileCache<Vec<Row>>> = LazyLock::new(FileCache::default);

pub fn collect_codex_from(root: &Path, start: DateTime<Utc>) -> Result<Vec<Row>> {
    let files = walk_ext(root, &["jsonl"]);
    let live: HashSet<PathBuf> = files.iter().cloned().collect();
    let mut rows = Vec::new();
    for file in files {
        if !modified_since(&file, start) {
            continue;
        }
        let Some(sig) = Sig::of(&file, 0) else {
            continue;
        };
        let parsed = CACHE.get_or_parse(&file, sig, || {
            read_lines(&file)
                .map(|lines| parse_session(lines, &file))
                .unwrap_or_default()
        });
        rows.extend(parsed.iter().cloned());
    }
    CACHE.prune_under(root, &live);
    Ok(rows)
}

/// Cumulative token counters of one Codex session.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Totals {
    input: u64,
    cached: u64,
    output: u64,
}

impl Totals {
    fn of(v: &Value) -> Totals {
        Totals {
            input: num(v.get("input_tokens")),
            cached: num(v.get("cached_input_tokens")),
            output: num(v.get("output_tokens")),
        }
    }
    fn is_zero(self) -> bool {
        self == Totals::default()
    }
}

/// Codex emits a `token_count` event after every model call carrying the
/// *cumulative* `total_token_usage` (and often a repeated, unchanged copy), so
/// per-call usage is the positive delta between successive totals. Sessions
/// without totals fall back to `last_token_usage`.
fn parse_session(lines: impl Iterator<Item = String>, file: &Path) -> Vec<Row> {
    let id = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut project: Option<String> = None;
    let mut model: Option<String> = None;
    let mut prev = Totals::default();
    let mut rows = Vec::new();

    for line in lines {
        let Ok(rec) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let payload = rec.get("payload");
        match rec.get("type").and_then(Value::as_str) {
            Some("session_meta") => {
                project = payload
                    .and_then(|p| p.get("cwd"))
                    .and_then(Value::as_str)
                    .and_then(dir_basename);
            }
            Some("turn_context") => {
                if let Some(m) = payload.and_then(|p| p.get("model")).and_then(Value::as_str) {
                    model = Some(m.to_string());
                }
            }
            Some("event_msg") => {}
            _ => continue,
        }
        let Some(p) =
            payload.filter(|p| p.get("type").and_then(Value::as_str) == Some("token_count"))
        else {
            continue;
        };
        let Some(info) = p.get("info").filter(|i| i.is_object()) else {
            continue;
        };
        if let Some(m) = info.get("model").and_then(Value::as_str) {
            model = Some(m.to_string());
        }
        let Some(ts) = rec
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(parse_ts)
        else {
            continue;
        };

        let delta = if let Some(t) = info.get("total_token_usage").filter(|t| t.is_object()) {
            let cur = Totals::of(t);
            let d = Totals {
                input: cur.input.saturating_sub(prev.input),
                cached: cur.cached.saturating_sub(prev.cached),
                output: cur.output.saturating_sub(prev.output),
            };
            prev = cur;
            d
        } else if let Some(l) = info.get("last_token_usage").filter(|l| l.is_object()) {
            Totals::of(l)
        } else {
            continue;
        };
        if delta.is_zero() {
            continue;
        }
        rows.push(Row {
            tool: Tool::Codex,
            project: project
                .clone()
                .or_else(|| model.clone())
                .unwrap_or_else(|| "unknown".into()),
            id: id.clone(),
            ts,
            // OpenAI counts cached tokens inside `input_tokens`: split them out.
            input: delta.input.saturating_sub(delta.cached),
            output: delta.output,
            cache_read: delta.cached,
            cache_write: 0,
            cost: 0.0,
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn meta(cwd: &str) -> String {
        json!({"timestamp":"2026-05-01T10:00:00Z","type":"session_meta","payload":{"cwd":cwd}})
            .to_string()
    }

    fn tc(ts: &str, total: (u64, u64, u64)) -> String {
        json!({"timestamp":ts,"type":"event_msg","payload":{"type":"token_count","info":{
            "total_token_usage":{"input_tokens":total.0,"cached_input_tokens":total.1,"output_tokens":total.2}}}})
        .to_string()
    }

    fn rows(lines: Vec<String>) -> Vec<Row> {
        parse_session(lines.into_iter(), Path::new("rollout-1.jsonl"))
    }

    #[test]
    fn cumulative_totals_become_per_call_deltas() {
        let r = rows(vec![
            meta("/home/me/app"),
            tc("2026-05-01T10:00:01Z", (100, 40, 10)),
            tc("2026-05-01T10:00:02Z", (100, 40, 10)), // repeated, unchanged → ignored
            tc("2026-05-01T10:00:03Z", (250, 90, 30)),
        ]);
        assert_eq!(r.len(), 2);
        // first call: 100 input of which 40 cached
        assert_eq!((r[0].input, r[0].cache_read, r[0].output), (60, 40, 10));
        // second call: +150 input of which +50 cached, +20 output
        assert_eq!((r[1].input, r[1].cache_read, r[1].output), (100, 50, 20));
        assert_eq!(r[0].project, "app");
        assert_eq!(r[0].tool, Tool::Codex);
    }

    #[test]
    fn the_sum_of_deltas_equals_the_final_total() {
        let r = rows(vec![
            tc("2026-05-01T10:00:01Z", (10, 0, 1)),
            tc("2026-05-01T10:00:02Z", (30, 5, 4)),
            tc("2026-05-01T10:00:03Z", (31, 5, 9)),
        ]);
        let input: u64 = r.iter().map(|x| x.input + x.cache_read).sum();
        let output: u64 = r.iter().map(|x| x.output).sum();
        assert_eq!((input, output), (31, 9));
    }

    #[test]
    fn falls_back_to_last_token_usage_without_totals() {
        let l = json!({"timestamp":"2026-05-01T10:00:00Z","type":"event_msg","payload":{"type":"token_count","info":{
            "model":"gpt-x","last_token_usage":{"input_tokens":7,"cached_input_tokens":2,"output_tokens":3}}}});
        let r = rows(vec![l.to_string()]);
        assert_eq!((r[0].input, r[0].cache_read, r[0].output), (5, 2, 3));
        assert_eq!(
            r[0].project, "gpt-x",
            "falls back to the model when there is no cwd"
        );
    }

    #[test]
    fn ignores_other_events_and_null_info() {
        let null_info = json!({"timestamp":"2026-05-01T10:00:00Z","type":"event_msg","payload":{"type":"token_count","info":null}});
        let other = json!({"timestamp":"2026-05-01T10:00:00Z","type":"event_msg","payload":{"type":"agent_message"}});
        assert!(rows(vec![
            null_info.to_string(),
            other.to_string(),
            "garbage".into()
        ])
        .is_empty());
    }

    #[test]
    fn model_from_turn_context_is_remembered() {
        let ctx = json!({"timestamp":"2026-05-01T10:00:00Z","type":"turn_context","payload":{"model":"gpt-5"}});
        let r = rows(vec![ctx.to_string(), tc("2026-05-01T10:00:01Z", (5, 0, 1))]);
        assert_eq!(r[0].project, "gpt-5");
    }

    #[test]
    fn collects_from_a_dated_directory_tree() {
        let d = tempfile::tempdir().unwrap();
        let day = d.path().join("2026/05/01");
        std::fs::create_dir_all(&day).unwrap();
        std::fs::write(
            day.join("rollout-a.jsonl"),
            tc("2026-05-01T10:00:01Z", (8, 0, 2)),
        )
        .unwrap();
        let r = collect_codex_from(d.path(), DateTime::<Utc>::UNIX_EPOCH).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "rollout-a");
    }
}
