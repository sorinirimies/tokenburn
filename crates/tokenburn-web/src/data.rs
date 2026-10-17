//! Loading and shaping the data shown on the dashboard.

use chrono::Local;
use tokenburn_core::utils::fmt::{compact, percent, thousands};
use tokenburn_core::{
    BucketSavings, Collected, Currency, Pie, Query, ReportRow, SavedInput, Savings, Selection,
    Summary, TimeSeries, collect, drill_priced,
};

/// Everything a page render needs.
#[derive(Debug, Clone)]
pub struct Dashboard {
    pub query: Query,
    pub total: Summary,
    /// What prompt caching, rtk and pi compaction saved in this selection.
    pub savings: Savings,
    /// Savings per chart bucket (minute, hour, day or month), oldest first.
    pub savings_by_bucket: Vec<BucketSavings>,
    /// US dollars per million input tokens the savings are valued at.
    pub input_price: f64,
    pub report: Vec<ReportRow>,
    pub projects: Vec<(String, Summary)>,
    pub warnings: Vec<String>,
    pub row_count: usize,
    /// Stacked per-tool totals over time (the bar chart).
    pub series: TimeSeries,
    /// Share by project (the donut).
    pub pie: Pie,
    /// The clicked bar and/or slice that everything above is scoped to (stale
    /// parts already dropped).
    pub selection: Selection,
    /// The display currency (costs in the data stay US dollars).
    pub currency: Currency,
    /// Local wall-clock time the snapshot was taken (`HH:MM:SS`).
    pub updated_at: String,
}

/// Shape collected rows into a [`Dashboard`] scoped to `selection` (pure — no I/O).
pub fn build_dashboard(
    query: Query,
    collected: Collected,
    selection: &Selection,
    currency: Currency,
    input_price: f64,
) -> Dashboard {
    let d = drill_priced(
        &collected.rows,
        &query,
        selection,
        &SavedInput {
            price: input_price,
            external: Some(&collected.external),
        },
    );
    Dashboard {
        query,
        total: d.total,
        savings: d.savings,
        savings_by_bucket: d.savings_by_bucket,
        input_price,
        report: d.report,
        series: d.series,
        pie: d.pie,
        projects: d.projects,
        selection: d.selection,
        currency,
        row_count: collected.rows.len(),
        updated_at: Local::now().format("%H:%M:%S").to_string(),
        warnings: collected.warnings,
    }
}

/// How often a connected page re-reads the logs.
pub const REFRESH_EVERY: std::time::Duration = std::time::Duration::from_secs(5);

/// Read the logs on a blocking thread and build the [`Dashboard`].
pub async fn load(query: Query, selection: &Selection, currency: Currency) -> Dashboard {
    let collected =
        tokio::task::spawn_blocking(move || collect(query.tool, query.window.start(Local::now())))
            .await
            .unwrap_or_else(|e| Collected {
                rows: Vec::new(),
                warnings: vec![format!("loader failed: {e}")],
                ..Default::default()
            });
    build_dashboard(
        query,
        collected,
        selection,
        currency,
        tokenburn_core::settings::input_price(),
    )
}

/// One printed row of the savings table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavingsRow {
    pub label: String,
    pub tokens: String,
    pub money: String,
    pub basis: String,
    pub total: bool,
}

/// The savings table: a row per source and a closing total (empty when nothing
/// was saved). `spent_usd` gives the share of the would-be cost.
pub fn savings_rows(sv: &Savings, currency: Currency, spent_usd: f64) -> Vec<SavingsRow> {
    if sv.is_empty() {
        return Vec::new();
    }
    let mut rows: Vec<SavingsRow> = sv
        .sources
        .iter()
        .map(|s| SavingsRow {
            label: s.source.label().to_string(),
            tokens: thousands(s.tokens),
            money: currency.format(s.usd),
            basis: format!("{} {}", thousands(s.events), s.source.event_noun()),
            total: false,
        })
        .collect();
    rows.push(SavingsRow {
        label: "total".to_string(),
        tokens: thousands(sv.tokens),
        money: currency.format(sv.usd),
        basis: sv
            .percent_of_would_be(spent_usd)
            .map(|p| format!("{} of the would-be cost", percent(p)))
            .unwrap_or_default(),
        total: true,
    });
    rows
}

/// One printed row of the per-interval savings table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketRow {
    pub label: String,
    pub tokens: String,
    pub money: String,
    /// `Prompt cache $9.00 · rtk $1.00`
    pub split: String,
}

/// The per-interval savings table, newest first, at most `limit` rows. The
/// interval is the chart's: a minute, hour, day or month.
pub fn bucket_rows(buckets: &[BucketSavings], currency: Currency, limit: usize) -> Vec<BucketRow> {
    buckets
        .iter()
        .rev()
        .take(limit)
        .map(|b| BucketRow {
            label: b.label.clone(),
            tokens: thousands(b.savings.tokens),
            money: currency.format(b.savings.usd),
            split: b
                .savings
                .sources
                .iter()
                .map(|s| format!("{} {}", s.source.label(), currency.format(s.usd)))
                .collect::<Vec<_>>()
                .join(" · "),
        })
        .collect()
}

/// `minute`, `hour`, `day` or `month`: the interval a query's savings are grouped by.
pub fn bucket_noun(query: &Query) -> &'static str {
    match tokenburn_core::chart_bucket(query) {
        tokenburn_core::Bucket::Minute => "minute",
        tokenburn_core::Bucket::Hour => "hour",
        tokenburn_core::Bucket::Day => "day",
        _ => "month",
    }
}

/// How many intervals the savings table lists.
pub const MAX_BUCKET_ROWS: usize = 24;

/// `compact` tokens + money, for the headline card's sub-label.
pub fn savings_headline(sv: &Savings, currency: Currency) -> String {
    format!("{} tok · {}", compact(sv.tokens), currency.format(sv.usd))
}

/// Width (0–100) of a share bar for `value` against `max`.
pub fn share_percent(value: u64, max: u64) -> u32 {
    if max == 0 {
        0
    } else {
        ((value as f64 / max as f64) * 100.0)
            .round()
            .clamp(0.0, 100.0) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokenburn_core::{Row, Tool};

    fn row(cost: f64) -> Row {
        Row {
            tool: Tool::Pi,
            project: "p".into(),
            id: "i".into(),
            ts: "2026-05-01T10:00:00Z".parse().unwrap(),
            input: 10,
            output: 5,
            cache_read: 0,
            cache_write: 0,
            cost,
        }
    }

    #[test]
    fn dashboard_totals_and_counts() {
        let c = Collected {
            rows: vec![row(0.5), row(0.25)],
            warnings: vec!["w".into()],
            ..Default::default()
        };
        let d = build_dashboard(
            Query::default(),
            c,
            &Selection::default(),
            Currency::USD,
            3.0,
        );
        assert_eq!(d.row_count, 2);
        assert_eq!(d.total.input, 20);
        assert_eq!(d.warnings.len(), 1);
        assert_eq!(d.updated_at.len(), 8, "HH:MM:SS");
        assert_eq!(d.report.last().unwrap().tool, "TOTAL");
    }

    #[test]
    fn empty_dashboard_has_no_report() {
        let d = build_dashboard(
            Query::default(),
            Collected::default(),
            &Selection::default(),
            Currency::USD,
            3.0,
        );
        assert!(d.report.is_empty() && d.projects.is_empty());
    }

    #[test]
    fn a_selection_scopes_the_dashboard_and_stale_ones_are_dropped() {
        let c = || Collected {
            rows: vec![row(0.5), row(0.25)],
            warnings: vec![],
            ..Default::default()
        };
        let sel = Selection {
            project: Some("pi:p".into()),
            ..Selection::default()
        };
        let d = build_dashboard(Query::default(), c(), &sel, Currency::USD, 3.0);
        assert_eq!(d.selection, sel);
        assert_eq!(d.total.turns, 2);

        let stale = Selection {
            project: Some("pi:ghost".into()),
            bucket: Some("nope".into()),
        };
        let d = build_dashboard(Query::default(), c(), &stale, Currency::USD, 3.0);
        assert!(d.selection.is_empty());
        assert_eq!(d.total.turns, 2);
    }

    #[test]
    fn share_percent_bounds() {
        assert_eq!(share_percent(0, 0), 0);
        assert_eq!(share_percent(50, 100), 50);
        assert_eq!(share_percent(200, 100), 100);
    }

    fn cached(read: u64) -> Row {
        Row {
            cache_read: read,
            ..row(0.5)
        }
    }

    fn external() -> tokenburn_core::ExternalSavings {
        tokenburn_core::ExternalSavings {
            rtk_tokens: 1_000_000,
            rtk_commands: 5,
            compaction_tokens: 0,
            compactions: 0,
            ..Default::default()
        }
    }

    #[test]
    fn the_dashboard_carries_savings_valued_at_the_price() {
        let c = Collected {
            rows: vec![cached(1_000_000), row(0.5)],
            external: external(),
            ..Default::default()
        };
        let d = build_dashboard(
            Query::default(),
            c,
            &Selection::default(),
            Currency::USD,
            10.0,
        );
        assert_eq!(d.input_price, 10.0);
        assert_eq!(d.savings.tokens, 2_000_000);
        assert!((d.savings.usd - 19.0).abs() < 1e-9, "9 cache + 10 rtk");
    }

    #[test]
    fn a_project_selection_keeps_only_that_projects_cache() {
        let mut other = cached(1_000_000);
        other.project = "other".into();
        let c = Collected {
            rows: vec![cached(2_000_000), other],
            external: external(),
            ..Default::default()
        };
        let sel = Selection {
            project: Some("pi:other".into()),
            ..Selection::default()
        };
        let d = build_dashboard(Query::default(), c, &sel, Currency::USD, 10.0);
        assert_eq!(
            d.savings.tokens, 1_000_000,
            "only the project's cache; rtk is not per project"
        );
    }

    #[test]
    fn savings_rows_have_a_line_per_source_and_a_total() {
        let c = Collected {
            rows: vec![cached(1_000_000)],
            external: external(),
            ..Default::default()
        };
        let d = build_dashboard(
            Query::default(),
            c,
            &Selection::default(),
            Currency::USD,
            10.0,
        );
        let rows = savings_rows(&d.savings, d.currency, d.total.cost_usd);
        let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, ["Prompt cache", "rtk", "total"]);
        assert_eq!(rows[0].money, "$9.00");
        assert_eq!(
            (rows[1].tokens.as_str(), rows[1].basis.as_str()),
            ("1,000,000", "5 commands")
        );
        assert!(rows[2].total && !rows[0].total);
        assert_eq!(rows[2].money, "$19.00");
        assert!(
            rows[2].basis.contains("97% of the would-be cost"),
            "{}",
            rows[2].basis
        );
        assert_eq!(
            savings_headline(&d.savings, d.currency),
            "2.00M tok · $19.00"
        );
        let eur = tokenburn_core::currency_by_code("EUR").unwrap();
        assert_eq!(savings_rows(&d.savings, eur, 0.5)[2].money, "€17.48");
    }

    #[test]
    fn no_savings_no_rows() {
        let d = build_dashboard(
            Query::default(),
            Collected {
                rows: vec![row(0.5)],
                ..Default::default()
            },
            &Selection::default(),
            Currency::USD,
            3.0,
        );
        assert!(d.savings.is_empty());
        assert!(savings_rows(&d.savings, d.currency, 0.5).is_empty());
    }

    fn at(ts: &str) -> chrono::DateTime<chrono::Utc> {
        ts.parse().unwrap()
    }

    /// Day 1: 1M cache tokens + a compaction; day 2: rtk.
    fn two_days() -> Collected {
        let mut day1 = cached(1_000_000);
        day1.ts = at("2026-05-01T10:00:00Z");
        let mut day2 = row(0.5);
        day2.ts = at("2026-05-02T10:00:00Z");
        Collected {
            rows: vec![day1, day2],
            external: tokenburn_core::ExternalSavings {
                rtk_tokens: 1_000_000,
                rtk_commands: 4,
                compaction_tokens: 1_000_000,
                compactions: 1,
                rtk_events: vec![(at("2026-05-02T10:00:00Z"), 1_000_000, 4)],
                compaction_events: vec![(at("2026-05-01T10:00:00Z"), 1_000_000)],
            },
            ..Default::default()
        }
    }

    #[test]
    fn savings_are_listed_per_interval_newest_first() {
        let d = build_dashboard(
            Query::default(),
            two_days(),
            &Selection::default(),
            Currency::USD,
            10.0,
        );
        let rows = bucket_rows(&d.savings_by_bucket, d.currency, 10);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].label > rows[1].label, "newest first");
        assert_eq!(
            (rows[0].money.as_str(), rows[0].split.as_str()),
            ("$10.00", "rtk $10.00")
        );
        assert_eq!(rows[1].money, "$19.00");
        assert_eq!(rows[1].split, "Prompt cache $9.00 · pi compaction $10.00");
        assert_eq!(
            bucket_rows(&d.savings_by_bucket, d.currency, 1).len(),
            1,
            "limit"
        );
        assert_eq!(bucket_noun(&d.query), "day", "month window charts by day");
    }

    #[test]
    fn the_interval_follows_the_bucket_filter() {
        let q = |bucket| Query {
            bucket,
            ..Query::default()
        };
        for (bucket, noun, rows) in [
            (tokenburn_core::Bucket::Minute, "minute", 2),
            (tokenburn_core::Bucket::Hour, "hour", 2),
            (tokenburn_core::Bucket::Day, "day", 2),
            (tokenburn_core::Bucket::Month, "month", 1),
        ] {
            let d = build_dashboard(
                q(bucket),
                two_days(),
                &Selection::default(),
                Currency::USD,
                10.0,
            );
            assert_eq!(bucket_noun(&d.query), noun);
            assert_eq!(d.savings_by_bucket.len(), rows, "{noun}");
        }
    }

    #[test]
    fn a_selected_bar_narrows_every_source_to_that_interval() {
        let day1 = tokenburn_core::bucket_label_at(
            at("2026-05-01T10:00:00Z"),
            tokenburn_core::Bucket::Day,
        );
        let sel = Selection {
            bucket: Some(day1),
            ..Selection::default()
        };
        let d = build_dashboard(Query::default(), two_days(), &sel, Currency::USD, 10.0);
        assert!(
            (d.savings.usd - 19.0).abs() < 1e-9,
            "cache + compaction, no rtk: {}",
            d.savings.usd
        );
        assert_eq!(
            d.savings_by_bucket.len(),
            2,
            "the list keeps every interval, like the bars"
        );
    }

    #[test]
    fn a_selected_project_drops_the_unattributed_sources() {
        let sel = Selection {
            project: Some("pi:p".into()),
            ..Selection::default()
        };
        let d = build_dashboard(Query::default(), two_days(), &sel, Currency::USD, 10.0);
        assert_eq!(d.savings.sources.len(), 1);
        assert!((d.savings.usd - 9.0).abs() < 1e-9);
    }
}
