//! Savings in detail: which tool saved the tokens, of what type, and where.
//!
//! The totals in [`super::Savings`] say *how much*. This says *what*: every saving
//! event with its source ([`SavingsSource`]), its **kind** (the type of thing that
//! was cut: an `rtk` command family, a squeeze output kind…) and its **origin**
//! (the directory it happened in). Events can be grouped by kind or by origin.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{pi_logs, rtk, SavingsSource};
use crate::features::pi;
use crate::features::usage::Tool;
use crate::ToolFilter;

/// Most events kept per source (newest first), so a year of history stays small.
pub const MAX_EVENTS_PER_SOURCE: usize = 5_000;

/// One saving.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetailEvent {
    pub ts: DateTime<Utc>,
    pub source: SavingsSource,
    /// The type of saving: `git` (rtk), `search` (squeeze), `stale tool results`…
    pub kind: String,
    /// Where it happened: a directory, or empty when unknown.
    pub origin: String,
    pub tokens: u64,
    /// What exactly: the command, the counts…
    pub note: String,
}

/// Events of one `(source, key)` pair, summed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetailGroup {
    pub source: SavingsSource,
    /// The kind or the origin, depending on how it was grouped.
    pub key: String,
    pub tokens: u64,
    pub events: u64,
}

/// Every saving in a window, newest first.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavingsDetail {
    pub events: Vec<DetailEvent>,
}

impl SavingsDetail {
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Savings per `(source, kind)`, biggest first.
    pub fn by_kind(&self) -> Vec<DetailGroup> {
        self.group(|e| e.kind.clone())
    }

    /// Savings per `(source, origin)`, biggest first.
    pub fn by_origin(&self) -> Vec<DetailGroup> {
        self.group(|e| {
            if e.origin.is_empty() {
                "(unknown)".to_string()
            } else {
                e.origin.clone()
            }
        })
    }

    fn group(&self, key: impl Fn(&DetailEvent) -> String) -> Vec<DetailGroup> {
        let mut map: BTreeMap<(SavingsSource, String), (u64, u64)> = BTreeMap::new();
        for e in &self.events {
            let v = map.entry((e.source, key(e))).or_default();
            v.0 += e.tokens;
            v.1 += 1;
        }
        let mut out: Vec<DetailGroup> = map
            .into_iter()
            .map(|((source, key), (tokens, events))| DetailGroup {
                source,
                key,
                tokens,
                events,
            })
            .collect();
        out.sort_by(|a, b| b.tokens.cmp(&a.tokens).then_with(|| a.key.cmp(&b.key)));
        out
    }

    /// Total tokens of one source.
    pub fn tokens_of(&self, source: SavingsSource) -> u64 {
        self.events
            .iter()
            .filter(|e| e.source == source)
            .map(|e| e.tokens)
            .sum()
    }
}

/// The command family of an rtk command: `rtk git status` → `git`.
pub fn rtk_kind(rtk_cmd: &str) -> String {
    let mut words = rtk_cmd.split_whitespace();
    let first = words.next().unwrap_or("");
    let word = if first == "rtk" {
        words.next().unwrap_or("")
    } else {
        first
    };
    if word.is_empty() {
        "other".to_string()
    } else {
        word.to_string()
    }
}

fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

fn newest_first_capped(mut v: Vec<DetailEvent>) -> Vec<DetailEvent> {
    v.sort_by_key(|e| std::cmp::Reverse(e.ts));
    v.truncate(MAX_EVENTS_PER_SOURCE);
    v
}

/// Build the detail from already-read sources (split out so tests need no disk).
pub fn build(
    rtk_entries: &[rtk::RtkEntry],
    compactions: &[pi::Compaction],
    diet: &[pi_logs::LogEntry],
    squeeze: &[pi_logs::LogEntry],
) -> SavingsDetail {
    let mut events = Vec::new();
    events.extend(newest_first_capped(
        rtk_entries
            .iter()
            .map(|e| DetailEvent {
                ts: e.ts,
                source: SavingsSource::Rtk,
                kind: rtk_kind(&e.rtk_cmd),
                origin: e.project_path.clone(),
                tokens: e.saved_tokens,
                note: format!(
                    "{} ({} → {} tok)",
                    clip(&e.original_cmd, 100),
                    e.input_tokens,
                    e.output_tokens
                ),
            })
            .collect(),
    ));
    events.extend(newest_first_capped(
        compactions
            .iter()
            .filter(|c| c.saved() > 0)
            .map(|c| DetailEvent {
                ts: c.ts,
                source: SavingsSource::PiCompaction,
                kind: "context compaction".into(),
                origin: String::new(),
                tokens: c.saved(),
                note: format!("{} → {} tok of context", c.tokens_before, c.tokens_after),
            })
            .collect(),
    ));
    events.extend(newest_first_capped(
        diet.iter()
            .map(|e| DetailEvent {
                ts: e.ts,
                source: SavingsSource::PiContextDiet,
                kind: "stale tool results".into(),
                origin: e.cwd.clone(),
                tokens: e.tokens,
                note: format!(
                    "{} results, {} args elided",
                    e.results.unwrap_or(0),
                    e.args.unwrap_or(0)
                ),
            })
            .collect(),
    ));
    events.extend(newest_first_capped(
        squeeze
            .iter()
            .map(|e| DetailEvent {
                ts: e.ts,
                source: SavingsSource::PiSqueeze,
                kind: e.kind.clone().unwrap_or_else(|| "output".into()),
                origin: e.cwd.clone(),
                tokens: e.tokens,
                note: "bash output shortened".into(),
            })
            .collect(),
    ));
    events.sort_by_key(|e| std::cmp::Reverse(e.ts));
    SavingsDetail { events }
}

/// Read the detail for `tool` since `start`, from the same places as
/// [`super::collect_external`] (rtk only counts for every tool, like there).
pub fn collect_detail(tool: ToolFilter, start: DateTime<Utc>) -> SavingsDetail {
    let rtk_entries = if tool == ToolFilter::All {
        rtk::entries_since(start).unwrap_or_default()
    } else {
        Vec::new()
    };
    if !tool.includes(Tool::Pi) {
        return build(&rtk_entries, &[], &[], &[]);
    }
    build(
        &rtk_entries,
        &pi::collect_compactions(start),
        &pi_logs::CONTEXT_DIET.entries_since(start),
        &pi_logs::SQUEEZE.entries_since(start),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(h: u32) -> DateTime<Utc> {
        format!("2026-01-02T{h:02}:00:00Z").parse().unwrap()
    }

    fn rtk_e(h: u32, cmd: &str, saved: u64, path: &str) -> rtk::RtkEntry {
        rtk::RtkEntry {
            ts: t(h),
            original_cmd: "orig".into(),
            rtk_cmd: cmd.into(),
            input_tokens: saved + 10,
            output_tokens: 10,
            saved_tokens: saved,
            project_path: path.into(),
        }
    }

    fn log(h: u32, tokens: u64, cwd: &str, kind: Option<&str>) -> pi_logs::LogEntry {
        pi_logs::LogEntry {
            ts: t(h),
            tokens,
            cwd: cwd.into(),
            kind: kind.map(str::to_string),
            results: Some(2),
            args: None,
        }
    }

    #[test]
    fn rtk_kind_is_the_command_family() {
        assert_eq!(rtk_kind("rtk git status"), "git");
        assert_eq!(rtk_kind("rtk ls"), "ls");
        assert_eq!(rtk_kind("cargo test"), "cargo");
        assert_eq!(rtk_kind("rtk"), "other");
        assert_eq!(rtk_kind(""), "other");
    }

    #[test]
    fn events_carry_source_kind_and_origin_newest_first() {
        let d = build(
            &[
                rtk_e(1, "rtk git log", 100, "/a"),
                rtk_e(5, "rtk ls", 50, "/b"),
            ],
            &[],
            &[log(3, 7, "/a", None)],
            &[log(4, 9, "/b", Some("search"))],
        );
        let order: Vec<_> = d.events.iter().map(|e| e.ts).collect();
        assert_eq!(order, [t(5), t(4), t(3), t(1)]);
        assert_eq!(d.events[1].kind, "search");
        assert_eq!(d.events[2].kind, "stale tool results");
        assert_eq!(d.events[2].note, "2 results, 0 args elided");
        assert_eq!(d.tokens_of(SavingsSource::Rtk), 150);
    }

    #[test]
    fn grouping_sums_and_sorts_biggest_first() {
        let d = build(
            &[
                rtk_e(1, "rtk git a", 10, "/a"),
                rtk_e(2, "rtk git b", 30, "/b"),
                rtk_e(3, "rtk ls", 25, ""),
            ],
            &[],
            &[],
            &[],
        );
        let k = d.by_kind();
        assert_eq!(
            (k[0].key.as_str(), k[0].tokens, k[0].events),
            ("git", 40, 2)
        );
        assert_eq!(k[1].key, "ls");
        let o = d.by_origin();
        assert_eq!(o[0].key, "/b");
        assert!(o.iter().any(|g| g.key == "(unknown)"));
    }

    #[test]
    fn compactions_without_savings_are_left_out_and_empty_is_empty() {
        let c = pi::Compaction {
            ts: t(1),
            tokens_before: 10,
            tokens_after: 10,
        };
        assert!(build(&[], &[c], &[], &[]).is_empty());
        let c = pi::Compaction {
            tokens_before: 100,
            ..c
        };
        let d = build(&[], &[c], &[], &[]);
        assert_eq!(d.events[0].tokens, 90);
    }

    #[test]
    fn long_commands_are_clipped_and_events_capped() {
        assert_eq!(clip("abcdef", 4), "abc…");
        assert_eq!(clip("abc", 4), "abc");
        let many: Vec<_> = (0..MAX_EVENTS_PER_SOURCE + 5)
            .map(|i| log((i % 24) as u32, 1, "/x", None))
            .collect();
        assert_eq!(
            build(&[], &[], &[], &many).events.len(),
            MAX_EVENTS_PER_SOURCE
        );
    }
}
