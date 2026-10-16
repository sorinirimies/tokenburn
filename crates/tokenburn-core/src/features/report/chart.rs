//! Chart data shared by every front-end: a stacked time series (per tool) and
//! the "share by project" pie. Front-ends only *draw* it — ratatui bars, iced
//! canvas, SVG — so all of them agree on the numbers.

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Timelike};

use super::types::{Bucket, Query, Window};
use crate::features::currency::Currency;
use crate::features::usage::{Row, Summary, Tool};
pub use crate::utils::fmt::{axis_tokens, short_label, short_name};

/// What a chart measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Metric {
    #[default]
    Tokens,
    /// Estimated `$` cost (only for tools that log one).
    Cost,
}

impl Metric {
    pub const ALL: [Metric; 2] = [Metric::Tokens, Metric::Cost];

    pub fn label(self) -> &'static str {
        match self {
            Metric::Tokens => "tokens",
            Metric::Cost => "cost",
        }
    }
}

impl std::fmt::Display for Metric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl std::str::FromStr for Metric {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "tokens" => Ok(Metric::Tokens),
            "cost" => Ok(Metric::Cost),
            other => Err(format!("must be tokens or cost (got '{other}')")),
        }
    }
}

/// At most this many buckets are returned (the most recent ones).
pub const MAX_BUCKETS: usize = 120;

/// Per-bucket, per-tool totals: `values[bucket][tool.series_index()]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TimeSeries {
    pub bucket: Bucket,
    /// One label per bucket, oldest first (`2026-05-01`, `2026-05`, `2026-05-01 13:00`).
    pub labels: Vec<String>,
    pub tokens: Vec<[u64; Tool::COUNT]>,
    pub cost: Vec<[f64; Tool::COUNT]>,
}

impl TimeSeries {
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    /// The value of `tool` in bucket `i` for `metric`.
    pub fn value(&self, metric: Metric, i: usize, tool: Tool) -> f64 {
        match metric {
            Metric::Tokens => self.tokens[i][tool.series_index()] as f64,
            Metric::Cost => self.cost[i][tool.series_index()],
        }
    }

    /// Total of all tools in bucket `i`.
    pub fn bucket_total(&self, metric: Metric, i: usize) -> f64 {
        Tool::ALL.iter().map(|t| self.value(metric, i, *t)).sum()
    }

    /// The tallest stacked bar — the y axis maximum.
    pub fn max_total(&self, metric: Metric) -> f64 {
        (0..self.labels.len())
            .map(|i| self.bucket_total(metric, i))
            .fold(0.0, f64::max)
    }

    /// Tools that have any value for `metric`, in canonical order (the legend).
    pub fn active_tools(&self, metric: Metric) -> Vec<Tool> {
        Tool::ALL
            .into_iter()
            .filter(|t| (0..self.labels.len()).any(|i| self.value(metric, i, *t) > 0.0))
            .collect()
    }
}

impl TimeSeries {
    /// The same series with every cost converted from US dollars to `currency`
    /// (tokens are untouched), ready to be drawn with that currency's axis labels.
    pub fn in_currency(&self, currency: &Currency) -> TimeSeries {
        let mut out = self.clone();
        for bucket in &mut out.cost {
            for v in bucket.iter_mut() {
                *v = currency.convert(*v);
            }
        }
        out
    }
}

/// Time-series charts always show a trend, so an un-bucketed query is charted at
/// the natural granularity of its window.
pub fn chart_bucket(query: &Query) -> Bucket {
    match query.bucket {
        Bucket::None => match query.window {
            Window::Today => Bucket::Hour,
            Window::Month => Bucket::Day,
            Window::All => Bucket::Month,
        },
        b => b,
    }
}

/// Start of the bucket containing `ts` (local time).
fn floor(ts: DateTime<Local>, bucket: Bucket) -> NaiveDate {
    // Hours are handled separately; this is the date part for day/month.
    match bucket {
        Bucket::Month | Bucket::None => ts.date_naive().with_day(1).unwrap_or(ts.date_naive()),
        _ => ts.date_naive(),
    }
}

fn label(ts: DateTime<Local>, bucket: Bucket) -> String {
    ts.format(bucket.format()).to_string()
}

/// The label of the chart bucket containing `row` — the same string
/// [`TimeSeries::labels`] uses, so a clicked bar can be matched to its rows.
pub fn bucket_label_of(row: &Row, bucket: Bucket) -> String {
    label(row.ts.with_timezone(&Local), bucket)
}

/// Build the stacked series for `rows` at [`chart_bucket`] granularity, with
/// empty buckets filled in so gaps in time show as gaps in the chart.
pub fn time_series(rows: &[Row], query: &Query) -> TimeSeries {
    let bucket = chart_bucket(query);
    let mut map: BTreeMap<String, ([u64; Tool::COUNT], [f64; Tool::COUNT])> = BTreeMap::new();
    let (mut first, mut last): (Option<DateTime<Local>>, Option<DateTime<Local>>) = (None, None);

    for r in rows {
        let ts = r.ts.with_timezone(&Local);
        first = Some(first.map_or(ts, |f| f.min(ts)));
        last = Some(last.map_or(ts, |l| l.max(ts)));
        let slot = map.entry(label(ts, bucket)).or_default();
        let i = r.tool.series_index();
        slot.0[i] += r.input + r.output + r.cache_read + r.cache_write;
        slot.1[i] += r.cost;
    }

    let (Some(first), Some(last)) = (first, last) else {
        return TimeSeries {
            bucket,
            ..Default::default()
        };
    };

    // Walk every bucket from the first to the last, whether or not it has rows.
    let mut labels = Vec::new();
    let mut cursor = start_of(first, bucket);
    let end = start_of(last, bucket);
    while cursor <= end && labels.len() < 100_000 {
        labels.push(label(cursor, bucket));
        cursor = next(cursor, bucket);
    }
    let skip = labels.len().saturating_sub(MAX_BUCKETS);
    let labels: Vec<String> = labels.into_iter().skip(skip).collect();

    let mut tokens = Vec::with_capacity(labels.len());
    let mut cost = Vec::with_capacity(labels.len());
    for l in &labels {
        let (t, c) = map.get(l).copied().unwrap_or_default();
        tokens.push(t);
        cost.push(c);
    }
    TimeSeries {
        bucket,
        labels,
        tokens,
        cost,
    }
}

/// Midnight / hour / first-of-month at or before `ts`.
fn start_of(ts: DateTime<Local>, bucket: Bucket) -> DateTime<Local> {
    let naive = match bucket {
        Bucket::Hour => ts.date_naive().and_hms_opt(ts.hour(), 0, 0),
        _ => floor(ts, bucket).and_hms_opt(0, 0, 0),
    };
    naive
        .and_then(|n| Local.from_local_datetime(&n).earliest())
        .unwrap_or(ts)
}

fn next(ts: DateTime<Local>, bucket: Bucket) -> DateTime<Local> {
    match bucket {
        Bucket::Hour => ts + Duration::hours(1),
        Bucket::Month | Bucket::None => {
            let (y, m) = if ts.month() == 12 {
                (ts.year() + 1, 1)
            } else {
                (ts.year(), ts.month() + 1)
            };
            Local
                .with_ymd_and_hms(y, m, 1, 0, 0, 0)
                .earliest()
                .unwrap_or(ts + Duration::days(31))
        }
        // Add a day to the *date* (not 24 h) so DST changes cannot skip or repeat one.
        Bucket::Day => ts
            .date_naive()
            .succ_opt()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .and_then(|n| Local.from_local_datetime(&n).earliest())
            .unwrap_or(ts + Duration::days(1)),
    }
}

// ── axes and geometry (shared by the GUI canvas and the web SVG) ───────────

/// A "nice" axis maximum ≥ `max`: 1, 2, 2.5, 5 or 10 × a power of ten.
pub fn nice_max(max: f64) -> f64 {
    if max <= 0.0 || !max.is_finite() {
        return 1.0;
    }
    let exp = max.log10().floor();
    let base = 10f64.powf(exp);
    let f = max / base;
    let nice = [1.0, 2.0, 2.5, 5.0, 10.0]
        .into_iter()
        .find(|n| *n >= f - 1e-9)
        .unwrap_or(10.0);
    nice * base
}

/// Axis label for a value: `1.5M` tokens or `$12.50`. A cost `v` must already be in
/// `currency` (see [`TimeSeries::in_currency`], [`Pie::in_currency`]).
pub fn axis_label(metric: Metric, v: f64, currency: &Currency) -> String {
    match metric {
        Metric::Tokens => axis_tokens(v),
        Metric::Cost => currency.axis(v),
    }
}

/// Which bucket is under `x` (relative to the plot's left edge)? `None` outside.
pub fn bucket_at(x: f32, plot_width: f32, buckets: usize) -> Option<usize> {
    if buckets == 0 || x < 0.0 || x >= plot_width || plot_width <= 0.0 {
        return None;
    }
    Some(((x / plot_width) * buckets as f32) as usize).filter(|i| *i < buckets)
}

/// `(start, end)` angles in radians (clockwise from 12 o'clock) for each slice.
pub fn slice_angles(values: &[f64]) -> Vec<(f32, f32)> {
    let total: f64 = values.iter().sum();
    if total <= 0.0 {
        return Vec::new();
    }
    let mut acc = 0.0;
    values
        .iter()
        .map(|v| {
            let start = acc / total;
            acc += v;
            let end = acc / total;
            (
                (start * std::f64::consts::TAU) as f32,
                (end * std::f64::consts::TAU) as f32,
            )
        })
        .collect()
}

// ── pie ─────────────────────────────────────────────────────────────────────

/// One pie slice.
#[derive(Debug, Clone, PartialEq)]
pub struct Slice {
    pub label: String,
    /// The tool, when the label starts with `tool:` (for colouring).
    pub tool: Option<Tool>,
    /// The full `tool:project` key (what [`super::Selection`] matches on);
    /// `None` for the folded `other` slice, which can't be selected.
    pub key: Option<String>,
    pub value: f64,
}

/// What the pie measures and its slices (largest first, tail folded into `other`).
#[derive(Debug, Clone, PartialEq)]
pub struct Pie {
    pub metric: Metric,
    pub slices: Vec<Slice>,
}

impl Pie {
    /// The same pie with a cost metric converted from US dollars to `currency`
    /// (a tokens pie is untouched). Slice proportions never change.
    pub fn in_currency(&self, currency: &Currency) -> Pie {
        let mut out = self.clone();
        if out.metric == Metric::Cost {
            for s in &mut out.slices {
                s.value = currency.convert(s.value);
            }
        }
        out
    }

    pub fn total(&self) -> f64 {
        self.slices.iter().map(|s| s.value).sum()
    }
}

/// Turn the per-project breakdown into at most `top_n` slices plus an `other`
/// slice for the remainder. The metric is cost when any entry has a cost, tokens
/// otherwise; zero-valued entries are dropped.
pub fn pie(projects: &[(String, Summary)], top_n: usize) -> Pie {
    let metric = if projects.iter().any(|(_, s)| s.cost_usd > 0.0) {
        Metric::Cost
    } else {
        Metric::Tokens
    };
    let value = |s: &Summary| match metric {
        Metric::Cost => s.cost_usd,
        Metric::Tokens => s.total_tokens as f64,
    };

    let mut slices: Vec<Slice> = projects
        .iter()
        .map(|(name, s)| Slice {
            label: short_name(name),
            tool: name
                .split_once(':')
                .and_then(|(t, _)| t.parse::<Tool>().ok()),
            key: Some(name.clone()),
            value: value(s),
        })
        .filter(|s| s.value > 0.0)
        .collect();
    slices.sort_by(|a, b| {
        b.value
            .partial_cmp(&a.value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let top_n = top_n.max(1);
    if slices.len() > top_n {
        let rest: f64 = slices.split_off(top_n).iter().map(|s| s.value).sum();
        slices.push(Slice {
            label: "other".into(),
            tool: None,
            key: None,
            value: rest,
        });
    }
    Pie { metric, slices }
}

/// `ts` as UTC from an RFC 3339 string.
#[cfg(test)]
fn utc(s: &str) -> chrono::DateTime<chrono::Utc> {
    s.parse().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(tool: Tool, ts: &str, tokens: u64, cost: f64) -> Row {
        Row {
            tool,
            project: "p".into(),
            id: "i".into(),
            ts: utc(ts),
            input: tokens,
            output: 0,
            cache_read: 0,
            cache_write: 0,
            cost,
        }
    }

    fn q(bucket: Bucket, window: Window) -> Query {
        Query {
            bucket,
            window,
            ..Query::default()
        }
    }

    #[test]
    fn empty_input_gives_an_empty_series() {
        let ts = time_series(&[], &q(Bucket::Day, Window::Month));
        assert!(ts.is_empty());
        assert_eq!(ts.max_total(Metric::Tokens), 0.0);
        assert!(ts.active_tools(Metric::Tokens).is_empty());
    }

    #[test]
    fn rows_are_stacked_per_bucket_and_tool() {
        let rows = [
            row(Tool::Pi, "2026-05-10T12:00:00Z", 100, 1.0),
            row(Tool::Pi, "2026-05-10T13:00:00Z", 50, 0.5),
            row(Tool::Claude, "2026-05-10T12:30:00Z", 25, 0.0),
        ];
        let ts = time_series(&rows, &q(Bucket::Day, Window::Month));
        assert_eq!(ts.labels.len(), 1);
        assert_eq!(ts.value(Metric::Tokens, 0, Tool::Pi), 150.0);
        assert_eq!(ts.value(Metric::Tokens, 0, Tool::Claude), 25.0);
        assert_eq!(ts.bucket_total(Metric::Tokens, 0), 175.0);
        assert!((ts.value(Metric::Cost, 0, Tool::Pi) - 1.5).abs() < 1e-9);
        assert_eq!(ts.active_tools(Metric::Tokens), [Tool::Pi, Tool::Claude]);
        assert_eq!(
            ts.active_tools(Metric::Cost),
            [Tool::Pi],
            "claude has no cost"
        );
    }

    #[test]
    fn gaps_between_the_first_and_last_day_are_filled_with_zeros() {
        let rows = [
            row(Tool::Pi, "2026-05-10T12:00:00Z", 10, 0.0),
            row(Tool::Pi, "2026-05-14T12:00:00Z", 20, 0.0),
        ];
        let ts = time_series(&rows, &q(Bucket::Day, Window::Month));
        assert_eq!(ts.labels.len(), 5, "10th…14th");
        let totals: Vec<f64> = (0..5).map(|i| ts.bucket_total(Metric::Tokens, i)).collect();
        assert_eq!(totals, [10.0, 0.0, 0.0, 0.0, 20.0]);
        assert!(ts.labels.windows(2).all(|w| w[0] < w[1]), "oldest first");
    }

    #[test]
    fn monthly_series_steps_over_year_ends() {
        let rows = [
            row(Tool::Pi, "2025-11-15T12:00:00Z", 1, 0.0),
            row(Tool::Pi, "2026-02-15T12:00:00Z", 1, 0.0),
        ];
        let ts = time_series(&rows, &q(Bucket::Month, Window::All));
        assert_eq!(ts.labels, ["2025-11", "2025-12", "2026-01", "2026-02"]);
    }

    #[test]
    fn hourly_series_steps_by_the_hour() {
        let rows = [
            row(Tool::Pi, "2026-05-10T10:05:00Z", 1, 0.0),
            row(Tool::Pi, "2026-05-10T13:55:00Z", 1, 0.0),
        ];
        let ts = time_series(&rows, &q(Bucket::Hour, Window::Today));
        // 3 h 50 min apart: 4 hour-buckets, or 5 in a half-hour-offset time zone.
        assert!((4..=5).contains(&ts.labels.len()), "{:?}", ts.labels);
    }

    #[test]
    fn an_unbucketed_query_is_charted_at_the_windows_natural_granularity() {
        assert_eq!(chart_bucket(&q(Bucket::None, Window::Today)), Bucket::Hour);
        assert_eq!(chart_bucket(&q(Bucket::None, Window::Month)), Bucket::Day);
        assert_eq!(chart_bucket(&q(Bucket::None, Window::All)), Bucket::Month);
        assert_eq!(
            chart_bucket(&q(Bucket::Hour, Window::All)),
            Bucket::Hour,
            "an explicit bucket is kept"
        );
    }

    #[test]
    fn only_the_most_recent_buckets_are_kept() {
        let rows = [
            row(Tool::Pi, "2020-01-01T12:00:00Z", 1, 0.0),
            row(Tool::Pi, "2026-05-14T12:00:00Z", 1, 0.0),
        ];
        let ts = time_series(&rows, &q(Bucket::Day, Window::All));
        assert_eq!(ts.labels.len(), MAX_BUCKETS);
        assert_eq!(ts.tokens.len(), MAX_BUCKETS);
        assert!(
            ts.labels.last().unwrap().starts_with("2026-05"),
            "the newest bucket survives"
        );
    }

    #[test]
    fn metric_parses_and_displays() {
        assert_eq!("COST".parse::<Metric>().unwrap(), Metric::Cost);
        assert!("bytes".parse::<Metric>().is_err());
        assert_eq!(Metric::Tokens.to_string(), "tokens");
    }

    fn s(cost: f64, tokens: u64) -> Summary {
        Summary {
            turns: 1,
            total_tokens: tokens,
            cost_usd: cost,
            ..Summary::default()
        }
    }

    #[test]
    fn pie_uses_cost_when_any_entry_has_one() {
        let p = pie(
            &[("pi:a".into(), s(2.0, 10)), ("zed:m".into(), s(0.0, 99))],
            6,
        );
        assert_eq!(p.metric, Metric::Cost);
        assert_eq!(p.slices.len(), 1, "zero-cost entries are dropped");
        assert_eq!(p.slices[0].tool, Some(Tool::Pi));
    }

    #[test]
    fn pie_falls_back_to_tokens() {
        let p = pie(
            &[("zed:m".into(), s(0.0, 50)), ("zed:n".into(), s(0.0, 70))],
            6,
        );
        assert_eq!(p.metric, Metric::Tokens);
        assert_eq!(p.slices[0].label, "zed:n", "largest first");
        assert_eq!(p.total(), 120.0);
    }

    #[test]
    fn pie_folds_the_tail_into_other() {
        let projects: Vec<_> = (1..=5)
            .map(|i| (format!("pi:p{i}"), s(f64::from(i), 1)))
            .collect();
        let p = pie(&projects, 3);
        assert_eq!(p.slices.len(), 4);
        assert_eq!(p.slices[3].label, "other");
        assert_eq!(p.slices[3].tool, None);
        assert!((p.slices[3].value - 3.0).abs() < 1e-9, "1 + 2");
    }

    #[test]
    fn nice_max_rounds_up_to_one_two_two_and_a_half_five_ten() {
        assert_eq!(nice_max(0.0), 1.0);
        assert_eq!(nice_max(f64::NAN), 1.0);
        assert_eq!(nice_max(0.9), 1.0);
        assert_eq!(nice_max(1.2), 2.0);
        assert_eq!(nice_max(2.3), 2.5);
        assert_eq!(nice_max(3.0), 5.0);
        assert_eq!(nice_max(7.0), 10.0);
        assert_eq!(nice_max(1_300_000.0), 2_000_000.0);
        assert_eq!(nice_max(12.0), 20.0);
        for v in [0.3, 1.0, 4.9, 99.0, 123_456.0, 8e9] {
            assert!(nice_max(v) >= v, "nice_max({v}) must not cut the data off");
        }
    }

    #[test]
    fn axis_labels_use_compact_numbers_and_dollars() {
        let usd = Currency::USD;
        assert_eq!(axis_label(Metric::Tokens, 1_500_000.0, &usd), "1.5M");
        assert_eq!(axis_label(Metric::Cost, 12.5, &usd), "$12.50");
        assert_eq!(axis_label(Metric::Cost, 0.25, &usd), "$0.25");
        assert_eq!(axis_label(Metric::Cost, 250.0, &usd), "$250");
        let eur = crate::features::currency::find("EUR").unwrap();
        assert_eq!(axis_label(Metric::Cost, 12.5, &eur), "€12.50");
        assert_eq!(
            axis_label(Metric::Tokens, 2_000.0, &eur),
            "2k",
            "tokens ignore the currency"
        );
    }

    #[test]
    fn hover_maps_x_to_a_bucket() {
        assert_eq!(bucket_at(0.0, 100.0, 4), Some(0));
        assert_eq!(bucket_at(24.9, 100.0, 4), Some(0));
        assert_eq!(bucket_at(25.0, 100.0, 4), Some(1));
        assert_eq!(bucket_at(99.9, 100.0, 4), Some(3));
        assert_eq!(bucket_at(100.0, 100.0, 4), None, "right edge is outside");
        assert_eq!(bucket_at(-1.0, 100.0, 4), None);
        assert_eq!(bucket_at(5.0, 100.0, 0), None);
    }

    #[test]
    fn slice_angles_cover_the_whole_circle_without_gaps() {
        let a = slice_angles(&[1.0, 1.0, 2.0]);
        assert_eq!(a.len(), 3);
        assert_eq!(a[0].0, 0.0);
        for w in a.windows(2) {
            assert!((w[0].1 - w[1].0).abs() < 1e-5, "slices must touch");
        }
        assert!((a[2].1 - std::f32::consts::TAU).abs() < 1e-4);
        assert!(
            (a[2].1 - a[2].0 - std::f32::consts::PI).abs() < 1e-4,
            "half the total = half the circle"
        );
        assert!(slice_angles(&[]).is_empty());
        assert!(slice_angles(&[0.0, 0.0]).is_empty());
    }

    #[test]
    fn an_empty_pie_has_no_slices() {
        assert!(pie(&[], 6).slices.is_empty());
    }

    #[test]
    fn converting_a_series_scales_costs_only() {
        let rows = [Row {
            tool: Tool::Pi,
            project: "p".into(),
            id: "i".into(),
            ts: utc("2026-05-10T12:00:00Z"),
            input: 1000,
            output: 0,
            cache_read: 0,
            cache_write: 0,
            cost: 10.0,
        }];
        let q = Query {
            bucket: Bucket::Day,
            window: Window::All,
            ..Query::default()
        };
        let usd = time_series(&rows, &q);
        let eur = crate::features::currency::find("EUR").unwrap();
        let conv = usd.in_currency(&eur);
        assert_eq!(conv.labels, usd.labels);
        assert_eq!(conv.tokens, usd.tokens, "tokens are not money");
        assert!((conv.bucket_total(Metric::Cost, 0) - 9.2).abs() < 1e-9);
        assert_eq!(usd.in_currency(&Currency::USD), usd, "USD is the identity");
    }

    #[test]
    fn converting_a_pie_scales_a_cost_pie_and_keeps_the_proportions() {
        let slice = |v: f64| Slice {
            label: "x".into(),
            tool: None,
            key: None,
            value: v,
        };
        let eur = crate::features::currency::find("EUR").unwrap();
        let cost = Pie {
            metric: Metric::Cost,
            slices: vec![slice(30.0), slice(10.0)],
        };
        let conv = cost.in_currency(&eur);
        assert!((conv.total() - 36.8).abs() < 1e-9);
        assert!((conv.slices[0].value / conv.total() - 0.75).abs() < 1e-9);
        let tokens = Pie {
            metric: Metric::Tokens,
            slices: vec![slice(300.0)],
        };
        assert_eq!(
            tokens.in_currency(&eur),
            tokens,
            "a tokens pie is untouched"
        );
    }
}
