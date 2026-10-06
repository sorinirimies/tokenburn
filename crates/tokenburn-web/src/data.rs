//! Loading and shaping the data shown on the dashboard.

use chrono::Local;
use tokenburn_core::{
    Collected, Pie, Query, ReportRow, Selection, Summary, TimeSeries, collect, drill,
};

/// Everything a page render needs.
#[derive(Debug, Clone)]
pub struct Dashboard {
    pub query: Query,
    pub total: Summary,
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
    /// Local wall-clock time the snapshot was taken (`HH:MM:SS`).
    pub updated_at: String,
}

/// Shape collected rows into a [`Dashboard`] scoped to `selection` (pure — no I/O).
pub fn build_dashboard(query: Query, collected: Collected, selection: &Selection) -> Dashboard {
    let d = drill(&collected.rows, &query, selection);
    Dashboard {
        query,
        total: d.total,
        report: d.report,
        series: d.series,
        pie: d.pie,
        projects: d.projects,
        selection: d.selection,
        row_count: collected.rows.len(),
        updated_at: Local::now().format("%H:%M:%S").to_string(),
        warnings: collected.warnings,
    }
}

/// How often a connected page re-reads the logs.
pub const REFRESH_EVERY: std::time::Duration = std::time::Duration::from_secs(5);

/// Read the logs on a blocking thread and build the [`Dashboard`].
pub async fn load(query: Query, selection: &Selection) -> Dashboard {
    let collected =
        tokio::task::spawn_blocking(move || collect(query.tool, query.window.start(Local::now())))
            .await
            .unwrap_or_else(|e| Collected {
                rows: Vec::new(),
                warnings: vec![format!("loader failed: {e}")],
            });
    build_dashboard(query, collected, selection)
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
        };
        let d = build_dashboard(Query::default(), c, &Selection::default());
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
        );
        assert!(d.report.is_empty() && d.projects.is_empty());
    }

    #[test]
    fn a_selection_scopes_the_dashboard_and_stale_ones_are_dropped() {
        let c = || Collected {
            rows: vec![row(0.5), row(0.25)],
            warnings: vec![],
        };
        let sel = Selection {
            project: Some("pi:p".into()),
            ..Selection::default()
        };
        let d = build_dashboard(Query::default(), c(), &sel);
        assert_eq!(d.selection, sel);
        assert_eq!(d.total.turns, 2);

        let stale = Selection {
            project: Some("pi:ghost".into()),
            bucket: Some("nope".into()),
        };
        let d = build_dashboard(Query::default(), c(), &stale);
        assert!(d.selection.is_empty());
        assert_eq!(d.total.turns, 2);
    }

    #[test]
    fn share_percent_bounds() {
        assert_eq!(share_percent(0, 0), 0);
        assert_eq!(share_percent(50, 100), 50);
        assert_eq!(share_percent(200, 100), 100);
    }
}
