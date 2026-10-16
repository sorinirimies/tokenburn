//! Click-to-drill-down: a [`Selection`] (one chart bucket and/or one project)
//! narrows everything on screen, and [`drill`] recomputes it.
//!
//! * clicking a **bar** scopes the table, the totals and the donut to that bucket;
//! * clicking a **slice** scopes the table, the totals and the bars to that project;
//! * both together intersect. Clicking the same thing again clears it.
//!
//! The chart you clicked in keeps showing everything (so you can pick another
//! bar or slice) with the selection highlighted.

use std::borrow::Cow;

use super::chart::{bucket_label_of, chart_bucket, pie, time_series, Pie, TimeSeries};
use super::ops::{build_report, by_project};
use super::types::{Query, ReportRow};
use crate::features::usage::{Row, Summary, Tool};

/// What the user has clicked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    /// A chart bucket label (`2026-09-22`, `2026-09`, `14:00` …).
    pub bucket: Option<String>,
    /// A full `tool:project` key.
    pub project: Option<String>,
}

impl Selection {
    pub fn is_empty(&self) -> bool {
        self.bucket.is_none() && self.project.is_none()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Select `label`, or clear it when it is already selected.
    pub fn toggle_bucket(&mut self, label: &str) {
        self.bucket = toggled(self.bucket.take(), label);
    }

    /// Select `key`, or clear it when it is already selected.
    pub fn toggle_project(&mut self, key: &str) {
        self.project = toggled(self.project.take(), key);
    }

    fn row_in_project(&self, row: &Row) -> bool {
        self.project
            .as_deref()
            .is_none_or(|p| project_key(row) == p)
    }

    fn row_in_bucket(&self, row: &Row, query: &Query) -> bool {
        self.bucket
            .as_deref()
            .is_none_or(|b| bucket_label_of(row, chart_bucket(query)) == b)
    }

    /// Human-readable chips, e.g. `["day 2026-09-22", "project pi:atlas"]`.
    pub fn describe(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(b) = &self.bucket {
            out.push(b.clone());
        }
        if let Some(p) = &self.project {
            out.push(super::chart::short_name(p));
        }
        out
    }
}

fn toggled(current: Option<String>, label: &str) -> Option<String> {
    if current.as_deref() == Some(label) {
        None
    } else {
        Some(label.to_string())
    }
}

/// The `tool:project` key of a row (the key [`by_project`] groups on).
pub fn project_key(row: &Row) -> String {
    format!("{}:{}", row.tool.as_str(), row.project)
}

/// Everything a dashboard shows, for one selection.
#[derive(Debug, Clone)]
pub struct Drill {
    pub report: Vec<ReportRow>,
    pub total: Summary,
    pub projects: Vec<(String, Summary)>,
    /// Bars: always the full time range, narrowed to the selected project.
    pub series: TimeSeries,
    /// Donut: narrowed to the selected bucket.
    pub pie: Pie,
    /// The selection after dropping parts that no longer exist in the data.
    pub selection: Selection,
}

/// Recompute the dashboard for `selection`. A bucket or project that is not in
/// `rows` any more (the data refreshed, the query changed) is dropped.
pub fn drill(rows: &[Row], query: &Query, selection: &Selection) -> Drill {
    let full = time_series(rows, query);
    let mut sel = selection.clone();
    if sel
        .bucket
        .as_ref()
        .is_some_and(|b| !full.labels.iter().any(|l| l == b))
    {
        sel.bucket = None;
    }
    if sel
        .project
        .as_ref()
        .is_some_and(|p| !rows.iter().any(|r| &project_key(r) == p))
    {
        sel.project = None;
    }

    // Rows are only copied when a selection actually narrows them. With nothing
    // selected (the normal case, on every refresh) all three views *borrow* the loaded
    // rows: copying a few hundred thousand of them every few seconds was pure waste.
    let in_project: Cow<[Row]> = if sel.project.is_none() {
        Cow::Borrowed(rows)
    } else {
        Cow::Owned(
            rows.iter()
                .filter(|r| sel.row_in_project(r))
                .cloned()
                .collect(),
        )
    };
    let in_bucket: Cow<[Row]> = if sel.bucket.is_none() {
        Cow::Borrowed(rows)
    } else {
        Cow::Owned(
            rows.iter()
                .filter(|r| sel.row_in_bucket(r, query))
                .cloned()
                .collect(),
        )
    };
    let both: Cow<[Row]> = match (sel.project.is_some(), sel.bucket.is_some()) {
        (false, false) => Cow::Borrowed(rows),
        (true, false) => Cow::Borrowed(&in_project),
        (false, true) => Cow::Borrowed(&in_bucket),
        (true, true) => Cow::Owned(
            in_project
                .iter()
                .filter(|r| sel.row_in_bucket(r, query))
                .cloned()
                .collect(),
        ),
    };

    // Bars keep the full x-axis even when a project narrows the values.
    let series = if sel.project.is_some() {
        restrict(&full, &in_project, query)
    } else {
        full
    };
    let projects = by_project(&both);
    Drill {
        report: build_report(&both, query),
        total: Summary::of(both.iter()),
        series,
        pie: pie(&by_project(&in_bucket), 8),
        projects,
        selection: sel,
    }
}

/// `full`'s labels with values recomputed from `rows`.
fn restrict(full: &TimeSeries, rows: &[Row], query: &Query) -> TimeSeries {
    let mut out = TimeSeries {
        bucket: full.bucket,
        labels: full.labels.clone(),
        tokens: vec![[0; Tool::COUNT]; full.labels.len()],
        cost: vec![[0.0; Tool::COUNT]; full.labels.len()],
    };
    let bucket = chart_bucket(query);
    for r in rows {
        let l = bucket_label_of(r, bucket);
        if let Some(i) = out.labels.iter().position(|x| *x == l) {
            let t = r.tool.series_index();
            out.tokens[i][t] += r.input + r.output + r.cache_read + r.cache_write;
            out.cost[i][t] += r.cost;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::report::types::{Bucket, Window};
    use crate::features::usage::Tool;

    fn row(tool: Tool, project: &str, ts: &str, tokens: u64) -> Row {
        Row {
            tool,
            project: project.into(),
            id: "x".into(),
            ts: ts.parse().unwrap(),
            input: tokens,
            output: 0,
            cache_read: 0,
            cache_write: 0,
            cost: tokens as f64 / 1000.0,
        }
    }

    fn rows() -> Vec<Row> {
        vec![
            row(Tool::Pi, "a", "2026-05-01T12:00:00Z", 1000),
            row(Tool::Pi, "b", "2026-05-01T13:00:00Z", 500),
            row(Tool::Claude, "a", "2026-05-03T12:00:00Z", 2000),
        ]
    }

    fn q() -> Query {
        Query {
            bucket: Bucket::Day,
            window: Window::All,
            ..Query::default()
        }
    }

    fn sel(bucket: Option<&str>, project: Option<&str>) -> Selection {
        Selection {
            bucket: bucket.map(Into::into),
            project: project.map(Into::into),
        }
    }

    #[test]
    fn no_selection_is_the_whole_dataset() {
        let d = drill(&rows(), &q(), &Selection::default());
        assert_eq!(d.total.total_tokens, 3500);
        assert_eq!(d.series.labels.len(), 3, "May 1, 2, 3 (gap filled)");
        assert!(d.selection.is_empty());
    }

    #[test]
    fn a_bucket_scopes_totals_report_and_donut_but_not_the_bars() {
        let all = drill(&rows(), &q(), &Selection::default());
        let first = all.series.labels[0].clone();
        let d = drill(&rows(), &q(), &sel(Some(&first), None));
        assert_eq!(d.total.total_tokens, 1500);
        assert!(d.report.iter().all(|r| r.bucket.as_deref() == Some(&first)));
        assert_eq!(d.pie.slices.len(), 2, "only the two projects of that day");
        assert_eq!(d.series.labels, all.series.labels, "x-axis unchanged");
        assert_eq!(
            d.series.bucket_total(crate::Metric::Tokens, 2),
            2000.0,
            "bars still show the other days"
        );
    }

    #[test]
    fn a_project_scopes_totals_report_and_bars_but_not_the_donut() {
        let all = drill(&rows(), &q(), &Selection::default());
        let d = drill(&rows(), &q(), &sel(None, Some("pi:a")));
        assert_eq!(d.total.total_tokens, 1000);
        assert_eq!(d.series.labels, all.series.labels, "same x-axis");
        assert_eq!(d.series.bucket_total(crate::Metric::Tokens, 0), 1000.0);
        assert_eq!(d.series.bucket_total(crate::Metric::Tokens, 2), 0.0);
        assert_eq!(
            d.pie.slices.len(),
            all.pie.slices.len(),
            "donut shows everything"
        );
    }

    #[test]
    fn both_intersect() {
        let all = drill(&rows(), &q(), &Selection::default());
        let first = all.series.labels[0].clone();
        let d = drill(&rows(), &q(), &sel(Some(&first), Some("pi:b")));
        assert_eq!(d.total.total_tokens, 500);
    }

    #[test]
    fn toggling_selects_then_clears() {
        let mut s = Selection::default();
        s.toggle_bucket("2026-05-01");
        assert_eq!(s.bucket.as_deref(), Some("2026-05-01"));
        s.toggle_bucket("2026-05-03");
        assert_eq!(
            s.bucket.as_deref(),
            Some("2026-05-03"),
            "another bar replaces it"
        );
        s.toggle_bucket("2026-05-03");
        assert!(s.bucket.is_none(), "same bar again clears");
        s.toggle_project("pi:a");
        assert!(!s.is_empty());
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn stale_selections_are_dropped() {
        let d = drill(&rows(), &q(), &sel(Some("1999-01-01"), Some("pi:ghost")));
        assert!(d.selection.is_empty());
        assert_eq!(d.total.total_tokens, 3500, "falls back to everything");
    }

    #[test]
    fn works_when_the_report_is_not_bucketed() {
        // Bucket::None charts per day (month window) but the table is per tool.
        let query = Query {
            bucket: Bucket::None,
            window: Window::Month,
            ..Query::default()
        };
        let all = drill(&rows(), &query, &Selection::default());
        let day = all.series.labels[0].clone();
        let d = drill(&rows(), &query, &sel(Some(&day), None));
        assert_eq!(d.total.total_tokens, 1500);
        assert!(d.report.iter().any(|r| r.tool == "TOTAL"));
    }

    #[test]
    fn describe_lists_the_chips() {
        assert!(Selection::default().describe().is_empty());
        let d = sel(Some("2026-05-01"), Some("pi:Users-me-atlas")).describe();
        assert_eq!(d, vec!["2026-05-01".to_string(), "pi:atlas".to_string()]);
    }

    #[test]
    fn slice_keys_identify_projects_and_other_is_unselectable() {
        let d = drill(&rows(), &q(), &Selection::default());
        assert!(d.pie.slices.iter().all(|s| s.key.is_some()));
        assert_eq!(project_key(&rows()[0]), "pi:a");
    }
}
