//! pi-context-diet's savings log.
//!
//! [pi-context-diet](https://github.com/sorinirimies/pi-context-diet) elides
//! stale tool results from pi's context and appends one JSON line per request
//! it trimmed to `context-diet-savings.jsonl` in pi's agent directory:
//!
//! ```json
//! {"ts":"2026-01-02T09:15:00.000Z","session":"…","cwd":"…","tokens":18400,"chars":55200,"results":12,"args":2}
//! ```
//!
//! Only `ts` and `tokens` are read. A missing file means "no savings"; lines
//! that do not parse are skipped.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use chrono::{DateTime, DurationRound, TimeDelta, Utc};
use serde::Deserialize;

/// File name of the log, inside pi's agent directory.
pub const LOG_FILE: &str = "context-diet-savings.jsonl";

/// `(minute, tokens kept out, requests)`, like rtk's events.
pub type DietEvent = (DateTime<Utc>, u64, u64);

/// Where the log is, resolved like pi-context-diet does (first hit wins):
///
/// 1. `TOKENBURN_CONTEXT_DIET_LOG` — explicit path override
/// 2. `$PI_CODING_AGENT_DIR/context-diet-savings.jsonl`
/// 3. `$XDG_CONFIG_HOME/pi/agent/context-diet-savings.jsonl`
/// 4. `~/.pi/agent/context-diet-savings.jsonl`
pub fn log_path() -> Option<PathBuf> {
    resolve_log_path(|k| std::env::var_os(k), dirs::home_dir())
}

/// [`log_path`] with the environment and home directory injected (for tests).
pub(crate) fn resolve_log_path(
    env: impl Fn(&str) -> Option<OsString>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    let non_empty = |k: &str| env(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    non_empty("TOKENBURN_CONTEXT_DIET_LOG").or_else(|| {
        non_empty("PI_CODING_AGENT_DIR")
            .or_else(|| non_empty("XDG_CONFIG_HOME").map(|d| d.join("pi").join("agent")))
            .or_else(|| home.map(|h| h.join(".pi").join("agent")))
            .map(|d| d.join(LOG_FILE))
    })
}

/// Requests trimmed since `start`, one entry per minute, oldest first.
pub fn events_since(start: DateTime<Utc>) -> Vec<DietEvent> {
    log_path().map_or_else(Vec::new, |p| events_in(&p, start))
}

#[derive(Deserialize)]
struct Line {
    ts: DateTime<Utc>,
    tokens: u64,
}

/// [`events_since`] for a specific log file; empty when it cannot be read.
pub fn events_in(path: &Path, start: DateTime<Utc>) -> Vec<DietEvent> {
    std::fs::read_to_string(path).map_or_else(|_| Vec::new(), |text| parse(&text, start))
}

/// The per-minute events in the log's `text` since `start`.
pub fn parse(text: &str, start: DateTime<Utc>) -> Vec<DietEvent> {
    let mut minutes: BTreeMap<DateTime<Utc>, (u64, u64)> = BTreeMap::new();
    for line in text.lines() {
        let Ok(l) = serde_json::from_str::<Line>(line) else {
            continue;
        };
        if l.ts < start || l.tokens == 0 {
            continue;
        }
        let minute = l.ts.duration_trunc(TimeDelta::minutes(1)).unwrap_or(l.ts);
        let e = minutes.entry(minute).or_default();
        e.0 += l.tokens;
        e.1 += 1;
    }
    minutes.into_iter().map(|(m, (t, n))| (m, t, n)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(h: u32, m: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 2, h, m, s).unwrap()
    }

    fn env_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
        move |k| {
            pairs
                .iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| OsString::from(*v))
        }
    }

    #[test]
    fn log_path_defaults_to_pi_agent_dir() {
        let p = resolve_log_path(env_of(&[]), Some(PathBuf::from("/home/u"))).unwrap();
        assert_eq!(p, PathBuf::from("/home/u/.pi/agent").join(LOG_FILE));
    }

    #[test]
    fn log_path_honours_overrides_in_order() {
        let home = Some(PathBuf::from("/h"));
        let all = [
            ("TOKENBURN_CONTEXT_DIET_LOG", "/x/log.jsonl"),
            ("PI_CODING_AGENT_DIR", "/agent"),
            ("XDG_CONFIG_HOME", "/xdg"),
        ];
        assert_eq!(
            resolve_log_path(env_of(&all), home.clone()).unwrap(),
            PathBuf::from("/x/log.jsonl")
        );
        assert_eq!(
            resolve_log_path(env_of(&all[1..]), home.clone()).unwrap(),
            PathBuf::from("/agent").join(LOG_FILE)
        );
        assert_eq!(
            resolve_log_path(env_of(&all[2..]), home.clone()).unwrap(),
            PathBuf::from("/xdg/pi/agent").join(LOG_FILE)
        );
        // Empty values are ignored, like unset ones.
        assert_eq!(
            resolve_log_path(env_of(&[("PI_CODING_AGENT_DIR", "")]), home).unwrap(),
            PathBuf::from("/h/.pi/agent").join(LOG_FILE)
        );
        assert_eq!(resolve_log_path(env_of(&[]), None), None);
    }

    #[test]
    fn groups_requests_per_minute_since_start() {
        let text = [
            r#"{"ts":"2026-01-02T08:59:59.000Z","tokens":999}"#,
            r#"{"ts":"2026-01-02T09:15:10.000Z","session":"s","cwd":"/w","tokens":100,"chars":300,"results":2,"args":0}"#,
            r#"{"ts":"2026-01-02T09:15:50.500Z","tokens":50}"#,
            r#"{"ts":"2026-01-02T09:17:00.000Z","tokens":7}"#,
        ]
        .join("\n");
        assert_eq!(
            parse(&text, at(9, 0, 0)),
            vec![(at(9, 15, 0), 150, 2), (at(9, 17, 0), 7, 1)]
        );
    }

    #[test]
    fn skips_broken_and_empty_lines() {
        let text = "not json\n\n{\"ts\":\"nope\",\"tokens\":1}\n{\"tokens\":5}\n\
                    {\"ts\":\"2026-01-02T09:00:00Z\",\"tokens\":0}\n\
                    {\"ts\":\"2026-01-02T09:00:00Z\",\"tokens\":3}\n{\"ts\":";
        assert_eq!(parse(text, at(0, 0, 0)), vec![(at(9, 0, 0), 3, 1)]);
    }

    #[test]
    fn missing_file_means_no_savings() {
        assert!(events_in(Path::new("/nonexistent/context-diet.jsonl"), at(0, 0, 0)).is_empty());
    }

    #[test]
    fn reads_a_real_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LOG_FILE);
        std::fs::write(&path, "{\"ts\":\"2026-01-02T10:00:00Z\",\"tokens\":42}\n").unwrap();
        assert_eq!(events_in(&path, at(0, 0, 0)), vec![(at(10, 0, 0), 42, 1)]);
    }
}
