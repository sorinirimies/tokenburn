use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::features::usage::{Row, Tool};
use crate::utils::cache::{FileCache, Sig};
use crate::utils::files::{
    env_path, modified_since, num, num_f, project_from_dirname, read_lines, split_paths, walk_ext,
};
use crate::utils::time::parse_ts;

/// Claude Code config roots (each holds a `projects/` directory).
///
/// `CLAUDE_CONFIG_DIR` (comma/`;`/`:` separated; a root or its `projects/`
/// directory) wins; otherwise `$XDG_CONFIG_HOME/claude` and `~/.claude`.
pub fn roots() -> Vec<PathBuf> {
    resolve_roots(
        std::env::var("CLAUDE_CONFIG_DIR").ok().as_deref(),
        env_path("XDG_CONFIG_HOME").or_else(|| dirs::home_dir().map(|h| h.join(".config"))),
        dirs::home_dir(),
    )
}

pub(crate) fn resolve_roots(
    env: Option<&str>,
    xdg_config: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut push = |p: PathBuf| {
        let projects = if p.file_name().is_some_and(|n| n == "projects") {
            p
        } else {
            p.join("projects")
        };
        if !out.contains(&projects) {
            out.push(projects);
        }
    };
    match env.filter(|e| !e.trim().is_empty()) {
        Some(list) => split_paths(list).into_iter().for_each(&mut push),
        None => {
            if let Some(x) = xdg_config {
                push(x.join("claude"));
            }
            if let Some(h) = home {
                push(h.join(".claude"));
            }
        }
    }
    out
}

/// Collect Claude Code usage from the default locations.
pub fn collect_claude(start: DateTime<Utc>) -> Result<Vec<Row>> {
    let mut all: HashMap<String, Cand> = HashMap::new();
    for root in roots().into_iter().filter(|r| r.is_dir()) {
        scan(&root, start, &mut all);
    }
    Ok(finish(all))
}

/// Collect from one `projects/` directory.
pub fn collect_claude_from(projects: &Path, start: DateTime<Utc>) -> Result<Vec<Row>> {
    let mut all = HashMap::new();
    scan(projects, start, &mut all);
    Ok(finish(all))
}

/// A candidate row plus what is needed to pick the best duplicate.
#[derive(Clone)]
struct Cand {
    row: Row,
    sidechain: bool,
}

fn finish(all: HashMap<String, Cand>) -> Vec<Row> {
    all.into_values().map(|c| c.row).collect()
}

/// Parsed `(dedupe key, candidate)` pairs per transcript, reused while the file
/// is unchanged. De-duplication ACROSS files happens after the cache, on every
/// refresh, because duplicates (sidechain replays) live in different files.
static CACHE: LazyLock<FileCache<Vec<(String, Cand)>>> = LazyLock::new(FileCache::default);

fn scan(projects: &Path, start: DateTime<Utc>, all: &mut HashMap<String, Cand>) {
    let files = walk_ext(projects, &["jsonl"]);
    let live: HashSet<PathBuf> = files.iter().cloned().collect();
    for file in files {
        if !modified_since(&file, start) {
            continue;
        }
        let Some(sig) = Sig::of(&file, 0) else {
            continue;
        };
        let parsed = CACHE.get_or_parse(&file, sig, || parse_file(projects, &file));
        for (key, cand) in parsed.iter() {
            insert_best(all, key.clone(), cand.clone());
        }
    }
    CACHE.prune_under(projects, &live);
}

fn parse_file(projects: &Path, file: &Path) -> Vec<(String, Cand)> {
    let project = project_of(projects, file);
    let id = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let Some(lines) = read_lines(file) else {
        return Vec::new();
    };
    lines
        .filter_map(|line| parse_line(&line, &project, &id))
        .collect()
}

/// Claude Code rewrites a streamed message several times (growing `output_tokens`)
/// and may replay a parent's messages into sidechain transcripts, so keep ONE
/// entry per message: the non-sidechain copy, else the one with the most tokens.
fn insert_best(all: &mut HashMap<String, Cand>, key: String, cand: Cand) {
    match all.get(&key) {
        None => {
            all.insert(key, cand);
        }
        Some(existing) => {
            let better = if existing.sidechain != cand.sidechain {
                existing.sidechain
            } else {
                total(&cand.row) > total(&existing.row)
            };
            if better {
                all.insert(key, cand);
            }
        }
    }
}

fn total(r: &Row) -> u64 {
    r.input + r.output + r.cache_read + r.cache_write
}

/// First path component below `projects/` (the encoded working directory).
fn project_of(projects: &Path, file: &Path) -> String {
    file.strip_prefix(projects)
        .ok()
        .and_then(|rel| rel.components().next())
        .map(|c| project_from_dirname(&c.as_os_str().to_string_lossy()))
        .unwrap_or_default()
}

/// The parts of a Claude Code transcript line tokenburn reads (the message
/// `content` — most of the bytes — is skipped without being allocated).
#[derive(Deserialize)]
struct Line {
    timestamp: Option<String>,
    #[serde(rename = "requestId")]
    request_id: Option<String>,
    #[serde(rename = "isApiErrorMessage")]
    is_api_error: Option<bool>,
    #[serde(rename = "isSidechain")]
    is_sidechain: Option<bool>,
    #[serde(rename = "costUSD")]
    cost_usd: Option<f64>,
    message: Option<Msg>,
}

#[derive(Deserialize)]
struct Msg {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: Option<f64>,
    output_tokens: Option<f64>,
    cache_creation_input_tokens: Option<f64>,
    cache_read_input_tokens: Option<f64>,
    /// Newer logs split cache writes by TTL (`ephemeral_5m_input_tokens`, …).
    cache_creation: Option<HashMap<String, Value>>,
}

fn parse_line(line: &str, project: &str, id: &str) -> Option<(String, Cand)> {
    // Lines without usage (user turns, tool results) are skipped before parsing.
    if !line.contains("\"usage\"") {
        return None;
    }
    let rec: Line = serde_json::from_str(line).ok()?;
    let msg = rec.message?;
    let usage = msg.usage?;
    if rec.is_api_error == Some(true) {
        return None;
    }
    // `<synthetic>` marks locally generated messages (no real API call).
    let model = msg.model?;
    if model == "<synthetic>" {
        return None;
    }
    let ts = parse_ts(&rec.timestamp?)?;

    let flat = num_f(usage.cache_creation_input_tokens);
    let cache_creation = if flat > 0 {
        flat
    } else {
        usage
            .cache_creation
            .map(|o| o.values().map(|v| num(Some(v))).sum())
            .unwrap_or(0)
    };
    let row = Row {
        tool: Tool::Claude,
        project: project.to_string(),
        id: id.to_string(),
        ts,
        input: num_f(usage.input_tokens),
        output: num_f(usage.output_tokens),
        cache_read: num_f(usage.cache_read_input_tokens),
        cache_write: cache_creation,
        cost: rec.cost_usd.unwrap_or(0.0),
    };
    if total(&row) == 0 {
        return None;
    }

    let key = match (msg.id.as_deref(), rec.request_id.as_deref()) {
        (Some(m), Some(r)) => format!("{m}\u{1}{r}"),
        // Without a request id, collapse repeats of one message in one session
        // at one instant; distinct timestamps stay separate.
        (Some(m), None) => format!("{m}\u{1}{id}\u{1}{}", ts.timestamp_millis()),
        // No ids at all: nothing to dedupe on.
        (None, _) => format!(
            "anon\u{1}{id}\u{1}{}\u{1}{}",
            ts.timestamp_millis(),
            total(&row)
        ),
    };
    let sidechain = rec.is_sidechain == Some(true);
    Some((key, Cand { row, sidechain }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    fn line(msg: &str, req: &str, out: u64, ts: &str) -> String {
        json!({
            "timestamp": ts, "requestId": req, "sessionId": "s1",
            "message": {"id": msg, "model": "claude-x", "usage": {
                "input_tokens": 10, "output_tokens": out,
                "cache_creation_input_tokens": 5, "cache_read_input_tokens": 100}}
        })
        .to_string()
    }

    fn write(dir: &Path, rel: &str, lines: &[String]) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, lines.join("\n")).unwrap();
    }

    fn epoch() -> DateTime<Utc> {
        DateTime::<Utc>::UNIX_EPOCH
    }

    #[test]
    fn parses_a_usage_line() {
        let l = line("m1", "r1", 20, "2026-05-01T10:00:00Z");
        let (_, c) = parse_line(&l, "proj", "sess").unwrap();
        assert_eq!(
            (
                c.row.input,
                c.row.output,
                c.row.cache_read,
                c.row.cache_write
            ),
            (10, 20, 100, 5)
        );
        assert_eq!(c.row.tool, Tool::Claude);
    }

    #[test]
    fn streamed_rewrites_keep_only_the_largest() {
        // Regression guard: the same message is logged several times with a
        // growing output_tokens — it must be counted once, at its final size.
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "-home-me-app/s1.jsonl",
            &[
                line("m1", "r1", 3, "2026-05-01T10:00:00Z"),
                line("m1", "r1", 40, "2026-05-01T10:00:01Z"),
                line("m1", "r1", 12, "2026-05-01T10:00:02Z"),
                line("m2", "r2", 7, "2026-05-01T10:01:00Z"),
            ],
        );
        let mut rows = collect_claude_from(d.path(), epoch()).unwrap();
        rows.sort_by_key(|r| r.output);
        assert_eq!(rows.iter().map(|r| r.output).collect::<Vec<_>>(), [7, 40]);
        assert_eq!(rows[0].project, "home-me-app");
    }

    #[test]
    fn duplicate_across_sessions_is_counted_once() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "p/a.jsonl",
            &[line("m1", "r1", 9, "2026-05-01T10:00:00Z")],
        );
        write(
            d.path(),
            "p/b.jsonl",
            &[line("m1", "r1", 9, "2026-05-01T10:00:00Z")],
        );
        assert_eq!(collect_claude_from(d.path(), epoch()).unwrap().len(), 1);
    }

    #[test]
    fn sidechain_replay_loses_to_the_parent_copy() {
        let mut replay: Value =
            serde_json::from_str(&line("m1", "r1", 99, "2026-05-01T10:00:00Z")).unwrap();
        replay["isSidechain"] = json!(true);
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "p/main.jsonl",
            &[line("m1", "r1", 9, "2026-05-01T10:00:00Z")],
        );
        write(d.path(), "p/sub/agent.jsonl", &[replay.to_string()]);
        let rows = collect_claude_from(d.path(), epoch()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].output, 9,
            "the parent copy wins even though the replay is bigger"
        );
    }

    #[test]
    fn skips_errors_synthetic_and_empty_usage() {
        let mut err: Value =
            serde_json::from_str(&line("e", "r", 5, "2026-05-01T10:00:00Z")).unwrap();
        err["isApiErrorMessage"] = json!(true);
        assert!(parse_line(&err.to_string(), "p", "i").is_none());
        let mut syn: Value =
            serde_json::from_str(&line("s", "r", 5, "2026-05-01T10:00:00Z")).unwrap();
        syn["message"]["model"] = json!("<synthetic>");
        assert!(parse_line(&syn.to_string(), "p", "i").is_none());
        let zero = json!({"timestamp":"2026-05-01T10:00:00Z","message":{"id":"z","model":"m","usage":{"input_tokens":0}}});
        assert!(parse_line(&zero.to_string(), "p", "i").is_none());
        assert!(parse_line("not json", "p", "i").is_none());
        assert!(parse_line(r#"{"type":"user","message":{"content":"hi"}}"#, "p", "i").is_none());
    }

    #[test]
    fn cache_creation_ttl_split_is_summed() {
        let l = json!({"timestamp":"2026-05-01T10:00:00Z","message":{"id":"m","model":"m","usage":{
            "input_tokens":1,"cache_creation":{"ephemeral_5m_input_tokens":4,"ephemeral_1h_input_tokens":6}}}});
        let (_, c) = parse_line(&l.to_string(), "p", "i").unwrap();
        assert_eq!(c.row.cache_write, 10);
    }

    #[test]
    fn cost_usd_is_kept_when_logged() {
        let l = json!({"timestamp":"2026-05-01T10:00:00Z","costUSD":0.5,"message":{"id":"m","model":"m","usage":{"input_tokens":1}}});
        assert!((parse_line(&l.to_string(), "p", "i").unwrap().1.row.cost - 0.5).abs() < 1e-9);
    }

    #[test]
    fn roots_default_to_xdg_then_home_and_honour_the_env() {
        let r = resolve_roots(None, Some("/x/.config".into()), Some("/h".into()));
        assert_eq!(
            r,
            [
                PathBuf::from("/x/.config/claude/projects"),
                PathBuf::from("/h/.claude/projects")
            ]
        );
        let r = resolve_roots(Some("/a, /b/projects"), None, Some("/h".into()));
        assert_eq!(
            r,
            [PathBuf::from("/a/projects"), PathBuf::from("/b/projects")]
        );
        assert_eq!(resolve_roots(Some("  "), None, Some("/h".into())).len(), 1);
    }
}
