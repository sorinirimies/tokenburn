use chrono::{DateTime, Utc};

use std::collections::BTreeMap;

use super::types::{
    sane_price, usd_for, BucketSavings, ExternalSavings, SavingsSource, SourceSavings,
};
use super::{rtk, CACHE_READ_PRICE_RATIO};
use crate::features::pi;
use crate::features::report::{bucket_label_at, bucket_label_of, chart_bucket, Query};
use crate::features::usage::{Row, Tool};
use crate::ToolFilter;

/// Prompt-cache saving over `rows`: every cache-read token was billed at
/// [`CACHE_READ_PRICE_RATIO`] of the input price, so the rest of the price is
/// saved. `events` counts the turns that hit the cache.
pub fn cache_savings<'a>(
    rows: impl IntoIterator<Item = &'a Row>,
    input_price_per_mtok: f64,
) -> SourceSavings {
    let (mut tokens, mut turns) = (0u64, 0u64);
    for r in rows.into_iter().filter(|r| r.cache_read > 0) {
        tokens += r.cache_read;
        turns += 1;
    }
    cache_savings_of(tokens, turns, input_price_per_mtok)
}

/// Money saved by `tokens` cache-read tokens at `input_price_per_mtok`.
pub fn cache_usd(tokens: u64, input_price_per_mtok: f64) -> f64 {
    cache_savings_of(tokens, 0, input_price_per_mtok).usd
}

/// Money saved by `tokens` tokens no longer carried (compaction, rtk) at `input_price_per_mtok`.
pub fn avoided_usd(tokens: u64, input_price_per_mtok: f64) -> f64 {
    usd_for(tokens, sane_price(input_price_per_mtok))
}

/// [`cache_savings`] from totals you already have (`cache_read` tokens over `turns`
/// cached turns), for callers that aggregated the rows themselves.
pub fn cache_savings_of(tokens: u64, turns: u64, input_price_per_mtok: f64) -> SourceSavings {
    SourceSavings {
        source: SavingsSource::PromptCache,
        tokens,
        usd: usd_for(tokens, sane_price(input_price_per_mtok)) * (1.0 - CACHE_READ_PRICE_RATIO),
        events: turns,
    }
}

/// Savings per chart bucket of `query` (minute, hour, day or month — the same
/// granularity as the chart and the report), oldest first. Buckets where nothing
/// was saved are left out. rtk and compactions land in the bucket they happened in.
pub fn savings_by_bucket(
    rows: &[Row],
    query: &Query,
    external: &ExternalSavings,
    input_price_per_mtok: f64,
) -> Vec<BucketSavings> {
    let bucket = chart_bucket(query);
    // Labels sort chronologically, so a BTreeMap keeps them in order.
    let mut cache: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.cache_read > 0) {
        let e = cache.entry(bucket_label_of(r, bucket)).or_default();
        e.0 += r.cache_read;
        e.1 += 1;
    }
    let mut labels: std::collections::BTreeSet<String> = cache.keys().cloned().collect();
    let mut ext: BTreeMap<String, ExternalSavings> = BTreeMap::new();
    for (ts, ..) in &external.rtk_events {
        labels.insert(bucket_label_at(*ts, bucket));
    }
    for (ts, _) in &external.compaction_events {
        labels.insert(bucket_label_at(*ts, bucket));
    }
    for l in &labels {
        ext.insert(
            l.clone(),
            external.within(|ts| &bucket_label_at(ts, bucket) == l),
        );
    }
    labels
        .into_iter()
        .map(|label| {
            let (tokens, turns) = cache.get(&label).copied().unwrap_or_default();
            let savings =
                crate::Savings::from_totals(tokens, turns, &ext[&label], input_price_per_mtok);
            BucketSavings { label, savings }
        })
        .filter(|b| !b.savings.is_empty())
        .collect()
}

/// Read the sources that live outside the usage rows, for `tool` since `start`.
///
/// rtk cannot say which agent ran a command, so its saving is only counted when
/// every tool is selected; pi's compactions belong to pi.
pub fn collect_external(tool: ToolFilter, start: DateTime<Utc>) -> ExternalSavings {
    let mut ext = ExternalSavings::default();
    if tool == ToolFilter::All {
        if let Some(events) = rtk::saved_events_since(start) {
            ext.rtk_tokens = events.iter().map(|e| e.1).sum();
            ext.rtk_commands = events.iter().map(|e| e.2).sum();
            ext.rtk_events = events;
        }
    }
    if tool.includes(Tool::Pi) {
        let compactions = pi::collect_compactions(start);
        ext.compaction_tokens = compactions.iter().map(|c| c.saved()).sum();
        ext.compactions = compactions.iter().filter(|c| c.saved() > 0).count() as u64;
        ext.compaction_events = compactions
            .iter()
            .filter(|c| c.saved() > 0)
            .map(|c| (c.ts, c.saved()))
            .collect();
    }
    ext
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Savings;

    fn row(cache_read: u64) -> Row {
        Row {
            tool: Tool::Pi,
            project: "p".into(),
            id: "i".into(),
            ts: "2026-05-01T00:00:00Z".parse().unwrap(),
            input: 1,
            output: 1,
            cache_read,
            cache_write: 0,
            cost: 0.0,
        }
    }

    #[test]
    fn cache_reads_are_credited_at_the_discount() {
        let rows = [row(1_000_000), row(0), row(1_000_000)];
        let s = cache_savings(&rows, 10.0);
        assert_eq!((s.tokens, s.events), (2_000_000, 2));
        assert!((s.usd - 18.0).abs() < 1e-9, "2M × $10 × 0.9: {}", s.usd);
        assert_eq!(cache_savings(&[], 10.0).tokens, 0);
    }

    #[test]
    fn totals_give_the_same_answer_as_rows() {
        let rows = [row(1_000_000), row(0), row(500_000)];
        let from_rows = Savings::build(&rows, &ExternalSavings::default(), 10.0);
        let from_totals = Savings::from_totals(1_500_000, 2, &ExternalSavings::default(), 10.0);
        assert_eq!(from_rows, from_totals);
    }

    #[test]
    fn a_bad_price_uses_the_default() {
        let s = cache_savings(&[row(1_000_000)], f64::NAN);
        assert!((s.usd - 2.7).abs() < 1e-9, "{}", s.usd);
    }

    #[test]
    fn build_combines_every_source() {
        let ext = ExternalSavings {
            rtk_tokens: 2_000_000,
            rtk_commands: 40,
            compaction_tokens: 1_000_000,
            compactions: 2,
            ..Default::default()
        };
        let sv = Savings::build(&[row(1_000_000)], &ext, 3.0);
        assert_eq!(sv.sources.len(), 3);
        assert_eq!(sv.tokens, 4_000_000);
        // 1M×3×0.9 + 2M×3 + 1M×3
        assert!((sv.usd - (2.7 + 6.0 + 3.0)).abs() < 1e-9, "{}", sv.usd);
        let rtk = sv.of(SavingsSource::Rtk).unwrap();
        assert_eq!((rtk.tokens, rtk.events), (2_000_000, 40));
        assert_eq!(sv.sources[0].source, SavingsSource::PromptCache);
    }

    #[test]
    fn build_with_nothing_is_empty() {
        assert!(Savings::build(&[], &ExternalSavings::default(), 3.0).is_empty());
    }

    #[test]
    fn rtk_is_only_counted_for_all_tools() {
        // Point rtk at a temp db; the env var is process-global, so keep this
        // the only test that touches it.
        let d = tempfile::tempdir().unwrap();
        let db = d.path().join("history.db");
        super::rtk::tests::make_db(&db, &[("2026-05-01T10:00:00+00:00", 700)]);
        std::env::set_var("TOKENBURN_RTK_DB", &db);
        let start = "2026-01-01T00:00:00Z".parse().unwrap();
        let all = collect_external(ToolFilter::All, start);
        assert_eq!((all.rtk_tokens, all.rtk_commands), (700, 1));
        let only = collect_external(ToolFilter::Only(Tool::Claude), start);
        assert_eq!(only.rtk_tokens, 0);
        std::env::remove_var("TOKENBURN_RTK_DB");
    }

    fn at(ts: &str) -> DateTime<Utc> {
        ts.parse().unwrap()
    }

    fn q(bucket: crate::Bucket) -> Query {
        Query {
            bucket,
            window: crate::Window::All,
            ..Query::default()
        }
    }

    fn timed_row(ts: &str, cache_read: u64) -> Row {
        Row {
            ts: at(ts),
            ..row(cache_read)
        }
    }

    fn ext() -> ExternalSavings {
        ExternalSavings {
            rtk_tokens: 300,
            rtk_commands: 3,
            compaction_tokens: 1_000_000,
            compactions: 1,
            rtk_events: vec![
                (at("2026-05-01T10:00:00Z"), 100, 1),
                (at("2026-05-01T10:01:00Z"), 200, 2),
            ],
            compaction_events: vec![(at("2026-05-02T09:00:00Z"), 1_000_000)],
        }
    }

    #[test]
    fn within_narrows_every_total_to_the_moments_kept() {
        let first_minute = ext().within(|ts| ts == at("2026-05-01T10:00:00Z"));
        assert_eq!(
            (first_minute.rtk_tokens, first_minute.rtk_commands),
            (100, 1)
        );
        assert_eq!(
            (first_minute.compaction_tokens, first_minute.compactions),
            (0, 0)
        );
        let none = ext().within(|_| false);
        assert_eq!(none, ExternalSavings::default());
        assert_eq!(ext().within(|_| true), ext());
    }

    #[test]
    fn savings_split_by_the_chart_interval() {
        use crate::Bucket;
        let rows = [
            timed_row("2026-05-01T10:00:30Z", 1_000_000),
            timed_row("2026-05-01T10:00:40Z", 1_000_000),
            timed_row("2026-05-03T10:00:00Z", 0),
        ];
        // A minute: 10:00 (cache ×2 + rtk), 10:01 (rtk), 05-02 09:00 (compaction).
        let m = savings_by_bucket(&rows, &q(Bucket::Minute), &ext(), 10.0);
        assert_eq!(m.len(), 3, "{m:?}");
        assert!(
            m.windows(2).all(|w| w[0].label < w[1].label),
            "oldest first"
        );
        // 2M × $10 × 0.9 + 100 tokens of rtk.
        assert!((m[0].savings.usd - (18.0 + 100.0 * 10.0 / 1e6)).abs() < 1e-9);
        assert_eq!(
            m[0].savings.of(SavingsSource::PromptCache).unwrap().events,
            2
        );
        assert_eq!(m[1].savings.sources.len(), 1, "rtk only");

        // A day: May 1 holds both rtk events and the cache; May 2 the compaction.
        let d = savings_by_bucket(&rows, &q(Bucket::Day), &ext(), 10.0);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].savings.of(SavingsSource::Rtk).unwrap().events, 3);
        assert!((d[1].savings.usd - 10.0).abs() < 1e-9);

        // A month: everything, and it adds up to the whole.
        let mo = savings_by_bucket(&rows, &q(Bucket::Month), &ext(), 10.0);
        assert_eq!(mo.len(), 1);
        let whole = Savings::build(&rows, &ext(), 10.0);
        assert_eq!(mo[0].savings.tokens, whole.tokens);
        assert!((mo[0].savings.usd - whole.usd).abs() < 1e-9);
    }

    #[test]
    fn intervals_with_nothing_saved_are_left_out() {
        let rows = [timed_row("2026-05-01T10:00:00Z", 0)];
        assert!(savings_by_bucket(
            &rows,
            &q(crate::Bucket::Day),
            &ExternalSavings::default(),
            3.0
        )
        .is_empty());
        assert!(savings_by_bucket(
            &[],
            &q(crate::Bucket::Day),
            &ExternalSavings::default(),
            3.0
        )
        .is_empty());
    }
}
