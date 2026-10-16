use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;

use crate::features::usage::{Row, Tool};
use crate::utils::cache::{FileCache, Sig};
use crate::utils::files::{modified_since, num, split_paths};

/// VS Code family editors whose `User/globalStorage` may hold the extensions' data.
const EDITORS: [&str; 6] = [
    "Code",
    "Code - Insiders",
    "VSCodium",
    "Cursor",
    "Windsurf",
    "Positron",
];

fn extension_id(tool: Tool) -> &'static str {
    match tool {
        Tool::Roo => "rooveterinaryinc.roo-cline",
        Tool::Kilo => "kilocode.kilo-code",
        _ => "saoudrizwan.claude-dev",
    }
}

fn env_key(tool: Tool) -> &'static str {
    match tool {
        Tool::Roo => "ROO_DATA_DIR",
        Tool::Kilo => "KILO_DATA_DIR",
        _ => "CLINE_DATA_DIR",
    }
}

/// `$<TOOL>_DATA_DIR` (list of `tasks` parents), else every editor's
/// `globalStorage/<extension>` (plus `~/.cline/data` for the Cline CLI).
pub fn roots(tool: Tool) -> Vec<PathBuf> {
    if let Some(list) = std::env::var(env_key(tool))
        .ok()
        .filter(|v| !v.trim().is_empty())
    {
        return split_paths(&list);
    }
    let mut out: Vec<PathBuf> = dirs::config_dir()
        .into_iter()
        .flat_map(|c| {
            EDITORS.iter().map(move |e| {
                c.join(e)
                    .join("User/globalStorage")
                    .join(extension_id(tool))
            })
        })
        .collect();
    if tool == Tool::Cline {
        out.extend(dirs::home_dir().map(|h| h.join(".cline/data")));
    }
    out
}

fn collect(tool: Tool, start: DateTime<Utc>) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    for root in roots(tool).into_iter().filter(|r| r.is_dir()) {
        rows.extend(collect_cline_from(tool, &root, start)?);
    }
    Ok(rows)
}

pub fn collect_cline(start: DateTime<Utc>) -> Result<Vec<Row>> {
    collect(Tool::Cline, start)
}
pub fn collect_roo(start: DateTime<Utc>) -> Result<Vec<Row>> {
    collect(Tool::Roo, start)
}
pub fn collect_kilo(start: DateTime<Utc>) -> Result<Vec<Row>> {
    collect(Tool::Kilo, start)
}

/// Parsed rows per task file, reused while the file is unchanged.
static CACHE: LazyLock<FileCache<Vec<Row>>> = LazyLock::new(FileCache::default);

/// Read every `<root>/tasks/<id>/ui_messages.json`.
pub fn collect_cline_from(tool: Tool, root: &Path, start: DateTime<Utc>) -> Result<Vec<Row>> {
    let tasks = root.join("tasks");
    let mut files = Vec::new();
    if let Ok(dir) = std::fs::read_dir(&tasks) {
        for e in dir.flatten() {
            let f = e.path().join("ui_messages.json");
            if f.is_file() {
                files.push(f);
            }
        }
    }
    files.sort();
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
            let id = file
                .parent()
                .and_then(Path::file_name)
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            std::fs::read_to_string(&file)
                .map(|text| parse_task(tool, &text, &id))
                .unwrap_or_default()
        });
        rows.extend(parsed.iter().filter(|r| r.ts >= start).cloned());
    }
    CACHE.prune_under(&tasks, &live);
    Ok(rows)
}

/// `api_req_started` messages carry the request's usage as a JSON string in `text`.
fn parse_task(tool: Tool, text: &str, id: &str) -> Vec<Row> {
    let Ok(Value::Array(msgs)) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    msgs.iter()
        .filter(|m| m.get("say").and_then(Value::as_str) == Some("api_req_started"))
        .filter_map(|m| {
            let u: Value = serde_json::from_str(m.get("text")?.as_str()?).ok()?;
            let input = num(u.get("tokensIn"));
            let output = num(u.get("tokensOut"));
            let cache_write = num(u.get("cacheWrites"));
            let cache_read = num(u.get("cacheReads"));
            if input + output + cache_write + cache_read == 0 {
                return None; // request still in flight
            }
            Some(Row {
                tool,
                project: tool.display_name().to_string(),
                id: id.to_string(),
                ts: Utc.timestamp_millis_opt(m.get("ts")?.as_i64()?).single()?,
                input,
                output,
                cache_read,
                cache_write,
                cost: u
                    .get("cost")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
                    .max(0.0),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TASK: &str = r#"[
      {"ts":1777629600000,"type":"say","say":"text","text":"hello"},
      {"ts":1777629601000,"type":"say","say":"api_req_started",
       "text":"{\"request\":\"x\",\"tokensIn\":100,\"tokensOut\":20,\"cacheWrites\":5,\"cacheReads\":30,\"cost\":0.0123}"},
      {"ts":1777629602000,"type":"say","say":"api_req_started","text":"{\"request\":\"pending\"}"},
      {"ts":1777629603000,"type":"say","say":"api_req_started","text":"not json"}]"#;

    #[test]
    fn reads_usage_from_api_requests_and_skips_pending_ones() {
        let r = parse_task(Tool::Roo, TASK, "t1");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].tool, Tool::Roo);
        assert_eq!(
            (r[0].input, r[0].output, r[0].cache_write, r[0].cache_read),
            (100, 20, 5, 30)
        );
        assert!((r[0].cost - 0.0123).abs() < 1e-9);
    }

    #[test]
    fn collects_every_task_below_tasks_dir() {
        let d = tempfile::tempdir().unwrap();
        let t = d.path().join("tasks/1700000000000");
        std::fs::create_dir_all(&t).unwrap();
        std::fs::write(t.join("ui_messages.json"), TASK).unwrap();
        let r = collect_cline_from(Tool::Cline, d.path(), DateTime::<Utc>::UNIX_EPOCH).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "1700000000000");
        assert!(collect_cline_from(
            Tool::Kilo,
            &d.path().join("none"),
            DateTime::<Utc>::UNIX_EPOCH
        )
        .unwrap()
        .is_empty());
    }

    #[test]
    fn malformed_files_yield_nothing() {
        assert!(parse_task(Tool::Cline, "{oops", "x").is_empty());
    }
}
