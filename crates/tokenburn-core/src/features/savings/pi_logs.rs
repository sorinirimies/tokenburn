//! The savings logs of pi plugins.
//!
//! Two of the author's pi plugins append one JSON line per saving to a log in
//! pi's agent directory, so usage trackers can count them over time:
//!
//! * [pi-context-diet](https://github.com/sorinirimies/pi-context-diet) elides
//!   stale tool results, one line per request it trimmed
//!   (`context-diet-savings.jsonl`);
//! * [pi-squeeze](https://github.com/sorinirimies/pi-squeeze) compacts bash
//!   output, one line per output it shortened (`squeeze-savings.jsonl`).
//!
//! ```json
//! {"ts":"2026-01-02T09:15:00.000Z","session":"…","cwd":"…","tokens":18400,"chars":55200,…}
//! ```
//!
//! Only `ts` and `tokens` are read. A missing file means "no savings"; lines
//! that do not parse are skipped.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use chrono::{DateTime, DurationRound, TimeDelta, Utc};
use serde::Deserialize;

/// `(minute, tokens kept out, savings)`, like rtk's events.
pub type LogEvent = (DateTime<Utc>, u64, u64);

/// One plugin's savings log: its file name in pi's agent directory, and the
/// environment variable that points tokenburn at another file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluginLog {
    pub file: &'static str,
    pub env_override: &'static str,
}

/// pi-context-diet's log: one line per request it trimmed.
pub const CONTEXT_DIET: PluginLog = PluginLog {
    file: "context-diet-savings.jsonl",
    env_override: "TOKENBURN_CONTEXT_DIET_LOG",
};

/// pi-squeeze's log: one line per bash output it shortened.
pub const SQUEEZE: PluginLog = PluginLog {
    file: "squeeze-savings.jsonl",
    env_override: "TOKENBURN_SQUEEZE_LOG",
};

impl PluginLog {
    /// Where the log is, resolved like the plugins do (first hit wins):
    ///
    /// 1. the override variable (`TOKENBURN_CONTEXT_DIET_LOG`, `TOKENBURN_SQUEEZE_LOG`)
    /// 2. `$PI_CODING_AGENT_DIR/<file>`
    /// 3. `$XDG_CONFIG_HOME/pi/agent/<file>`
    /// 4. `~/.pi/agent/<file>`
    pub fn path(self) -> Option<PathBuf> {
        self.resolve(|k| std::env::var_os(k), dirs::home_dir())
    }

    /// [`PluginLog::path`] with the environment and home directory injected (for tests).
    pub(crate) fn resolve(
        self,
        env: impl Fn(&str) -> Option<OsString>,
        home: Option<PathBuf>,
    ) -> Option<PathBuf> {
        let non_empty = |k: &str| env(k).filter(|v| !v.is_empty()).map(PathBuf::from);
        non_empty(self.env_override).or_else(|| {
            non_empty("PI_CODING_AGENT_DIR")
                .or_else(|| non_empty("XDG_CONFIG_HOME").map(|d| d.join("pi").join("agent")))
                .or_else(|| home.map(|h| h.join(".pi").join("agent")))
                .map(|d| d.join(self.file))
        })
    }

    /// Savings since `start`, one entry per minute, oldest first.
    pub fn events_since(self, start: DateTime<Utc>) -> Vec<LogEvent> {
        self.path().map_or_else(Vec::new, |p| events_in(&p, start))
    }
}

#[derive(Deserialize)]
struct Line {
    ts: DateTime<Utc>,
    tokens: u64,
}

/// [`PluginLog::events_since`] for a specific log file; empty when it cannot be read.
pub fn events_in(path: &Path, start: DateTime<Utc>) -> Vec<LogEvent> {
    std::fs::read_to_string(path).map_or_else(|_| Vec::new(), |text| parse(&text, start))
}

/// The per-minute events in the log's `text` since `start`.
pub fn parse(text: &str, start: DateTime<Utc>) -> Vec<LogEvent> {
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
    fn logs_default_to_pi_agent_dir() {
        let home = Some(PathBuf::from("/home/u"));
        for log in [CONTEXT_DIET, SQUEEZE] {
            let p = log.resolve(env_of(&[]), home.clone()).unwrap();
            assert_eq!(p, PathBuf::from("/home/u/.pi/agent").join(log.file));
        }
        assert_ne!(CONTEXT_DIET.file, SQUEEZE.file);
        assert_ne!(CONTEXT_DIET.env_override, SQUEEZE.env_override);
    }

    #[test]
    fn log_path_honours_overrides_in_order() {
        let home = Some(PathBuf::from("/h"));
        let all = [
            ("TOKENBURN_CONTEXT_DIET_LOG", "/x/log.jsonl"),
            ("PI_CODING_AGENT_DIR", "/agent"),
            ("XDG_CONFIG_HOME", "/xdg"),
        ];
        let diet = CONTEXT_DIET;
        assert_eq!(
            diet.resolve(env_of(&all), home.clone()).unwrap(),
            PathBuf::from("/x/log.jsonl")
        );
        // Each log has its own override: the context-diet one does not move squeeze's.
        assert_eq!(
            SQUEEZE.resolve(env_of(&all), home.clone()).unwrap(),
            PathBuf::from("/agent").join(SQUEEZE.file)
        );
        assert_eq!(
            diet.resolve(env_of(&all[1..]), home.clone()).unwrap(),
            PathBuf::from("/agent").join(diet.file)
        );
        assert_eq!(
            diet.resolve(env_of(&all[2..]), home.clone()).unwrap(),
            PathBuf::from("/xdg/pi/agent").join(diet.file)
        );
        // Empty values are ignored, like unset ones.
        assert_eq!(
            diet.resolve(env_of(&[("PI_CODING_AGENT_DIR", "")]), home)
                .unwrap(),
            PathBuf::from("/h/.pi/agent").join(diet.file)
        );
        assert_eq!(diet.resolve(env_of(&[]), None), None);
    }

    #[test]
    fn groups_savings_per_minute_since_start() {
        let text = [
            r#"{"ts":"2026-01-02T08:59:59.000Z","tokens":999}"#,
            r#"{"ts":"2026-01-02T09:15:10.000Z","session":"s","cwd":"/w","tokens":100,"chars":300,"results":2,"args":0}"#,
            r#"{"ts":"2026-01-02T09:15:50.500Z","tokens":50,"kind":"build"}"#,
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
        assert!(events_in(Path::new("/nonexistent/savings.jsonl"), at(0, 0, 0)).is_empty());
    }

    #[test]
    fn reads_a_real_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SQUEEZE.file);
        std::fs::write(&path, "{\"ts\":\"2026-01-02T10:00:00Z\",\"tokens\":42}\n").unwrap();
        assert_eq!(events_in(&path, at(0, 0, 0)), vec![(at(10, 0, 0), 42, 1)]);
    }
}
