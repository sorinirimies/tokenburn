use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::features::usage::{Row, Tool};
use crate::utils::cache::{FileCache, Sig};
use crate::utils::files::{modified_since, num_f, read_lines, walk_ext};
use crate::utils::time::parse_ts;

/// Where pi stores its sessions, honouring pi's own overrides.
///
/// Resolution order (first hit wins):
///
/// 1. `TOKENBURN_PI_SESSIONS` — tokenburn-specific escape hatch
/// 2. `PI_CODING_AGENT_SESSION_DIR` — pi's session-directory override
/// 3. `$PI_CODING_AGENT_DIR/sessions` — pi's agent-directory override
/// 4. `~/.pi/agent/sessions` (`%USERPROFILE%\.pi\agent\sessions` on Windows)
pub fn sessions_dir() -> Option<PathBuf> {
    resolve_sessions_dir(|k| std::env::var_os(k), dirs::home_dir())
}

/// [`sessions_dir`] with the environment and home directory injected (for tests).
pub(crate) fn resolve_sessions_dir(
    env: impl Fn(&str) -> Option<std::ffi::OsString>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    let non_empty = |k: &str| env(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    non_empty("TOKENBURN_PI_SESSIONS")
        .or_else(|| non_empty("PI_CODING_AGENT_SESSION_DIR"))
        .or_else(|| non_empty("PI_CODING_AGENT_DIR").map(|d| d.join("sessions")))
        .or_else(|| home.map(|h| h.join(".pi").join("agent").join("sessions")))
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

/// Parsed rows per session file, reused while the file is unchanged.
static CACHE: LazyLock<FileCache<Vec<Row>>> = LazyLock::new(FileCache::default);

/// Collect pi usage rows from every `*.jsonl` below `root`.
///
/// Files not modified since `start` are skipped as a cheap pre-filter; exact
/// per-turn filtering by timestamp is left to the caller. Unchanged files are
/// served from a per-process cache, so repeated refreshes only re-parse the
/// session that is still growing.
pub fn collect_pi_from(root: &Path, start: DateTime<Utc>) -> Result<Vec<Row>> {
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
        let parsed = CACHE.get_or_parse(&file, sig, || parse_file(&file));
        rows.extend(parsed.iter().filter(|r| r.ts >= start).cloned());
    }
    CACHE.prune_under(root, &live);
    Ok(rows)
}

/// Parse one session file, streaming it line by line (session files can be
/// hundreds of megabytes — never hold one in memory).
fn parse_file(file: &Path) -> Vec<Row> {
    let Some(lines) = read_lines(file) else {
        tracing::warn!("pi: cannot read {}", file.display());
        return Vec::new();
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
    lines
        .filter_map(|l| parse_line(&l, &project, &id))
        .collect()
}

/// The only parts of a pi session line tokenburn reads. Everything else — above
/// all the message `content`, which is most of the bytes — is skipped without
/// being allocated.
#[derive(Deserialize)]
struct PiLine {
    timestamp: Option<String>,
    message: Option<PiMessage>,
}

#[derive(Deserialize)]
struct PiMessage {
    usage: Option<PiUsage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PiUsage {
    input: Option<f64>,
    output: Option<f64>,
    cache_read: Option<f64>,
    cache_write: Option<f64>,
    cost: Option<PiCost>,
}

#[derive(Deserialize)]
struct PiCost {
    total: Option<f64>,
}

/// Parse one jsonl line into a [`Row`]; `None` unless it carries
/// `message.usage` and a top-level `timestamp`.
pub(crate) fn parse_line(line: &str, project: &str, id: &str) -> Option<Row> {
    // Most lines (user turns, tool results) carry no usage: skip them without
    // touching a JSON parser.
    if !line.contains("\"usage\"") {
        return None;
    }
    let rec: PiLine = serde_json::from_str(line).ok()?;
    let usage = rec.message?.usage?;
    let ts = parse_ts(&rec.timestamp?)?;
    Some(Row {
        tool: Tool::Pi,
        project: project.to_string(),
        id: id.to_string(),
        ts,
        input: num_f(usage.input),
        output: num_f(usage.output),
        cache_read: num_f(usage.cache_read),
        cache_write: num_f(usage.cache_write),
        cost: usage.cost.and_then(|c| c.total).unwrap_or(0.0),
    })
}

/// One pi context compaction: the session's old messages were replaced by a
/// summary, so every later turn carries fewer tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Compaction {
    pub ts: DateTime<Utc>,
    /// Context size before compacting (pi's own `tokensBefore`).
    pub tokens_before: u64,
    /// Context size of the first turn after it, or an estimate from the summary
    /// text when the session ended right there.
    pub tokens_after: u64,
}

impl Compaction {
    /// Tokens no longer carried by every following turn.
    pub fn saved(&self) -> u64 {
        self.tokens_before.saturating_sub(self.tokens_after)
    }
}

/// Parsed compactions per session file, cached like the usage rows.
static COMPACTION_CACHE: LazyLock<FileCache<Vec<Compaction>>> = LazyLock::new(FileCache::default);

/// Compactions from the default sessions directory (none when pi is absent).
pub fn collect_compactions(start: DateTime<Utc>) -> Vec<Compaction> {
    match sessions_dir() {
        Some(dir) if dir.is_dir() => collect_compactions_from(&dir, start),
        _ => Vec::new(),
    }
}

/// Every compaction at or after `start` in the `*.jsonl` files below `root`.
pub fn collect_compactions_from(root: &Path, start: DateTime<Utc>) -> Vec<Compaction> {
    let files = walk_ext(root, &["jsonl"]);
    let live: HashSet<PathBuf> = files.iter().cloned().collect();
    let mut out = Vec::new();
    for file in files {
        if !modified_since(&file, start) {
            continue;
        }
        let Some(sig) = Sig::of(&file, 0) else {
            continue;
        };
        let parsed = COMPACTION_CACHE.get_or_parse(&file, sig, || parse_compactions(&file));
        out.extend(parsed.iter().filter(|c| c.ts >= start).copied());
    }
    COMPACTION_CACHE.prune_under(root, &live);
    out
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompactionLine {
    timestamp: Option<String>,
    summary: Option<String>,
    tokens_before: Option<f64>,
}

fn parse_compactions(file: &Path) -> Vec<Compaction> {
    let Some(lines) = read_lines(file) else {
        return Vec::new();
    };
    let mut out: Vec<Compaction> = Vec::new();
    // A compaction waiting for the next turn, whose usage shows the new context size.
    let mut pending = false;
    for line in lines {
        if line.contains("\"type\":\"compaction\"") {
            if let Some(c) = parse_compaction_line(&line) {
                out.push(c);
                pending = true;
            }
        } else if pending {
            if let Some(row) = parse_line(&line, "", "") {
                if let Some(c) = out.last_mut() {
                    c.tokens_after = row.input + row.cache_read + row.cache_write;
                }
                pending = false;
            }
        }
    }
    out
}

/// A compaction entry; `tokens_after` is estimated from the summary (≈4 bytes
/// per token) until a following turn replaces it.
pub(crate) fn parse_compaction_line(line: &str) -> Option<Compaction> {
    let rec: CompactionLine = serde_json::from_str(line).ok()?;
    Some(Compaction {
        ts: parse_ts(&rec.timestamp?)?,
        tokens_before: num_f(rec.tokens_before),
        tokens_after: rec.summary.map_or(0, |s| s.len() as u64 / 4),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Tests that look at the process-wide cache's counters must not interleave.
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

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

    fn env_of<'a>(
        pairs: &'a [(&'a str, &'a str)],
    ) -> impl Fn(&str) -> Option<std::ffi::OsString> + 'a {
        move |k| {
            pairs
                .iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| std::ffi::OsString::from(*v))
        }
    }

    #[test]
    fn sessions_dir_defaults_to_home() {
        let d = resolve_sessions_dir(env_of(&[]), Some(PathBuf::from("/home/u"))).unwrap();
        assert_eq!(d, PathBuf::from("/home/u/.pi/agent/sessions"));
    }

    #[test]
    fn sessions_dir_honours_pi_overrides_in_order() {
        let home = Some(PathBuf::from("/home/u"));
        let d = resolve_sessions_dir(env_of(&[("PI_CODING_AGENT_DIR", "/agent")]), home.clone());
        assert_eq!(d.unwrap(), PathBuf::from("/agent/sessions"));

        let d = resolve_sessions_dir(
            env_of(&[
                ("PI_CODING_AGENT_DIR", "/agent"),
                ("PI_CODING_AGENT_SESSION_DIR", "/sess"),
            ]),
            home.clone(),
        );
        assert_eq!(d.unwrap(), PathBuf::from("/sess"));

        let d = resolve_sessions_dir(
            env_of(&[
                ("PI_CODING_AGENT_SESSION_DIR", "/sess"),
                ("TOKENBURN_PI_SESSIONS", "/mine"),
            ]),
            home,
        );
        assert_eq!(d.unwrap(), PathBuf::from("/mine"));
    }

    #[test]
    fn empty_override_is_ignored() {
        let d = resolve_sessions_dir(
            env_of(&[("TOKENBURN_PI_SESSIONS", "")]),
            Some(PathBuf::from("/h")),
        );
        assert_eq!(d.unwrap(), PathBuf::from("/h/.pi/agent/sessions"));
    }

    #[test]
    fn unchanged_files_are_not_parsed_again_but_growing_ones_are() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("--cache-proj--");
        fs::create_dir_all(&proj).unwrap();
        let f = proj.join("s.jsonl");
        fs::write(&f, format!("{LINE}\n")).unwrap();
        let start = "1970-01-01T00:00:00Z".parse().unwrap();

        let (h0, m0) = CACHE.stats();
        assert_eq!(collect_pi_from(dir.path(), start).unwrap().len(), 1);
        assert_eq!(collect_pi_from(dir.path(), start).unwrap().len(), 1);
        let (h1, m1) = CACHE.stats();
        assert_eq!(
            (h1 - h0, m1 - m0),
            (1, 1),
            "second refresh served from the cache"
        );

        // The session grows (a new turn is appended): it must be re-parsed and counted.
        fs::write(&f, format!("{LINE}\n{LINE}\n")).unwrap();
        assert_eq!(collect_pi_from(dir.path(), start).unwrap().len(), 2);
        let (_, m2) = CACHE.stats();
        assert_eq!(m2 - m1, 1);
    }

    #[test]
    fn deleted_files_leave_the_cache() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("p");
        fs::create_dir_all(&proj).unwrap();
        let f = proj.join("s.jsonl");
        fs::write(&f, format!("{LINE}\n")).unwrap();
        let start = "1970-01-01T00:00:00Z".parse().unwrap();
        assert_eq!(collect_pi_from(dir.path(), start).unwrap().len(), 1);
        fs::remove_file(&f).unwrap();
        assert!(collect_pi_from(dir.path(), start).unwrap().is_empty());
    }

    #[test]
    fn collects_from_directory() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("--home-me-proj--");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("s.jsonl"), format!("{LINE}\ngarbage\n{LINE}\n")).unwrap();
        let start = "1970-01-01T00:00:00Z".parse().unwrap();
        let rows = collect_pi_from(dir.path(), start).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].project, "home-me-proj");
    }

    const COMPACTION: &str = r#"{"type":"compaction","timestamp":"2026-05-01T11:00:00Z","summary":"0123456789012345678901234567890123456789","tokensBefore":1000}"#;
    const AFTER: &str = r#"{"timestamp":"2026-05-01T11:01:00Z","message":{"usage":{"input":100,"output":5,"cacheRead":200,"cacheWrite":50,"cost":{"total":0.1}}}}"#;

    #[test]
    fn compaction_savings_use_the_next_turns_context() {
        let c = parse_compaction_line(COMPACTION).unwrap();
        assert_eq!(
            (c.tokens_before, c.tokens_after, c.saved()),
            (1000, 10, 990)
        );
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("s.jsonl");
        fs::write(&f, format!("{LINE}\n{COMPACTION}\nnoise\n{AFTER}\n")).unwrap();
        let got = parse_compactions(&f);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].tokens_after, 350);
        assert_eq!(got[0].saved(), 650);
    }

    #[test]
    fn compaction_at_the_end_of_a_session_falls_back_to_the_summary_estimate() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("s.jsonl");
        fs::write(&f, format!("{COMPACTION}\n")).unwrap();
        assert_eq!(parse_compactions(&f)[0].saved(), 990);
        // Growth past the context never goes negative.
        let big = Compaction {
            ts: parse_ts("2026-05-01T11:00:00Z").unwrap(),
            tokens_before: 5,
            tokens_after: 9,
        };
        assert_eq!(big.saved(), 0);
    }

    #[test]
    fn bad_compaction_lines_are_ignored() {
        assert!(parse_compaction_line("nope").is_none());
        assert!(parse_compaction_line(r#"{"type":"compaction","tokensBefore":1}"#).is_none());
    }

    #[test]
    fn compactions_are_collected_by_window_and_cached() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("--cp--");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("s.jsonl"), format!("{COMPACTION}\n{AFTER}\n")).unwrap();
        let all = "1970-01-01T00:00:00Z".parse().unwrap();
        let after = "2026-06-01T00:00:00Z".parse().unwrap();
        assert_eq!(collect_compactions_from(dir.path(), all).len(), 1);
        let (h0, _) = COMPACTION_CACHE.stats();
        assert_eq!(collect_compactions_from(dir.path(), all).len(), 1);
        assert_eq!(COMPACTION_CACHE.stats().0 - h0, 1, "served from the cache");
        assert!(collect_compactions_from(dir.path(), after).is_empty());
        fs::remove_file(proj.join("s.jsonl")).unwrap();
        assert!(collect_compactions_from(dir.path(), all).is_empty());
    }
}
