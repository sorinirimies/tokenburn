use std::collections::BTreeMap;

use chrono::Local;

use super::types::{Bucket, Query, ReportRow, ToolFilter};
use crate::features::usage::{Row, Summary};

/// Aggregate `rows` according to `query`.
///
/// * bucketed: one line per `(bucket, tool)`, sorted by bucket then tool;
/// * un-bucketed: a per-tool breakdown plus `TOTAL` when `query.tool` is
///   `all`, otherwise just `TOTAL`.
pub fn build_report(rows: &[Row], query: &Query) -> Vec<ReportRow> {
    if rows.is_empty() {
        return Vec::new();
    }

    if query.bucket != Bucket::None {
        let fmt = query.bucket.format();
        let mut groups: BTreeMap<(String, &'static str), Vec<&Row>> = BTreeMap::new();
        for r in rows {
            let label = r.ts.with_timezone(&Local).format(fmt).to_string();
            groups.entry((label, r.tool.as_str())).or_default().push(r);
        }
        return groups
            .into_iter()
            .map(|((bucket, tool), rs)| ReportRow {
                bucket: Some(bucket),
                tool: tool.to_string(),
                summary: Summary::of(rs),
            })
            .collect();
    }

    let total = ReportRow {
        bucket: None,
        tool: "TOTAL".into(),
        summary: Summary::of(rows),
    };
    if query.tool != ToolFilter::All {
        return vec![total];
    }

    let mut per_tool: BTreeMap<&'static str, Vec<&Row>> = BTreeMap::new();
    for r in rows {
        per_tool.entry(r.tool.as_str()).or_default().push(r);
    }
    let mut out: Vec<ReportRow> = per_tool
        .into_iter()
        .map(|(tool, rs)| ReportRow {
            bucket: None,
            tool: tool.to_string(),
            summary: Summary::of(rs),
        })
        .collect();
    out.push(total);
    out
}

/// Per-project (or per-model, where a tool logs no project) breakdown, most expensive first.
/// Whether a projects / models breakdown tells something the report does not:
/// true when some tool has more than one project (or model) behind it. With one
/// per tool the breakdown would repeat the report's tool rows.
pub fn projects_add_detail(report: &[ReportRow], projects: &[(String, Summary)]) -> bool {
    let tools: std::collections::BTreeSet<&str> = report
        .iter()
        .map(|r| r.tool.as_str())
        .filter(|t| *t != "TOTAL")
        .collect();
    let tools = if tools.is_empty() {
        projects
            .iter()
            .filter_map(|(k, _)| k.split(':').next())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    } else {
        tools.len()
    };
    projects.len() > tools
}

pub fn by_project(rows: &[Row]) -> Vec<(String, Summary)> {
    let mut groups: BTreeMap<(&'static str, &str), Vec<&Row>> = BTreeMap::new();
    for r in rows {
        groups
            .entry((r.tool.as_str(), r.project.as_str()))
            .or_default()
            .push(r);
    }
    let mut out: Vec<(String, Summary)> = groups
        .into_iter()
        .map(|((tool, project), rs)| (format!("{tool}:{project}"), Summary::of(rs)))
        .collect();
    out.sort_by(|a, b| {
        b.1.cost_usd
            .partial_cmp(&a.1.cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.1.total_tokens.cmp(&a.1.total_tokens))
    });
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn projects_are_detail_only_when_a_tool_has_several() {
        use super::*;
        let rr = |tool: &str| ReportRow {
            bucket: None,
            tool: tool.into(),
            summary: Summary::default(),
        };
        let p = |k: &str| (k.to_string(), Summary::default());
        let report = [rr("pi"), rr("claude"), rr("TOTAL")];
        assert!(!projects_add_detail(&report, &[p("pi:a"), p("claude:b")]));
        assert!(projects_add_detail(
            &report,
            &[p("pi:a"), p("pi:b"), p("claude:b")]
        ));
        assert!(!projects_add_detail(&[], &[]));
        // No tool rows to compare with: tools come from the project keys.
        assert!(!projects_add_detail(&[], &[p("pi:a")]));
        assert!(projects_add_detail(&[], &[p("pi:a"), p("pi:b")]));
    }

    use super::*;
    use crate::features::report::types::Window;
    use crate::features::usage::Tool;

    fn row(tool: Tool, ts: &str, input: u64, cost: f64) -> Row {
        Row {
            tool,
            project: "p".into(),
            id: "i".into(),
            ts: ts.parse().unwrap(),
            input,
            output: 1,
            cache_read: 0,
            cache_write: 0,
            cost,
        }
    }

    #[test]
    fn total_with_breakdown_for_all() {
        let rows = vec![
            row(Tool::Pi, "2026-05-01T12:00:00Z", 10, 0.5),
            row(Tool::Zed, "2026-05-02T12:00:00Z", 20, 0.0),
        ];
        let rep = build_report(&rows, &Query::default());
        let names: Vec<_> = rep.iter().map(|r| r.tool.as_str()).collect();
        assert_eq!(names, ["pi", "zed", "TOTAL"]);
        assert_eq!(rep[2].summary.input, 30);
        assert_eq!(rep[2].summary.total_tokens, 32);
    }

    #[test]
    fn single_tool_only_total() {
        let rows = vec![row(Tool::Pi, "2026-05-01T12:00:00Z", 10, 0.5)];
        let q = Query {
            tool: ToolFilter::PI,
            ..Query::default()
        };
        assert_eq!(build_report(&rows, &q).len(), 1);
    }

    #[test]
    fn monthly_buckets_group() {
        let rows = vec![
            row(Tool::Pi, "2026-05-10T12:00:00Z", 1, 0.1),
            row(Tool::Pi, "2026-05-20T12:00:00Z", 1, 0.1),
            row(Tool::Pi, "2026-07-10T12:00:00Z", 1, 0.1),
        ];
        let q = Query::from_flags(ToolFilter::All, false, false, false, true, false);
        assert_eq!(q.window, Window::All);
        let rep = build_report(&rows, &q);
        assert_eq!(rep.len(), 2);
        assert_eq!(rep[0].summary.turns, 2);
    }

    #[test]
    fn minutely_buckets_group_by_the_minute() {
        let rows = vec![
            row(Tool::Pi, "2026-05-10T12:00:05Z", 1, 0.1),
            row(Tool::Pi, "2026-05-10T12:00:55Z", 1, 0.1),
            row(Tool::Pi, "2026-05-10T12:01:05Z", 1, 0.1),
        ];
        let q = Query::from_flags(ToolFilter::All, true, false, false, false, false);
        let rep = build_report(&rows, &q);
        assert_eq!(rep.len(), 2);
        assert_eq!(rep[0].summary.turns, 2);
        assert!(rep[0].bucket.as_deref().unwrap().len() == 16);
    }

    #[test]
    fn empty_rows_empty_report() {
        assert!(build_report(&[], &Query::default()).is_empty());
    }

    #[test]
    fn flags_precedence_and_windows() {
        let q = Query::from_flags(ToolFilter::All, false, true, true, true, false);
        assert_eq!((q.bucket, q.window), (Bucket::Hour, Window::Today));
        let q = Query::from_flags(ToolFilter::All, false, false, true, false, true);
        assert_eq!((q.bucket, q.window), (Bucket::Day, Window::All));
        let q = Query::from_flags(ToolFilter::All, true, true, true, true, false);
        assert_eq!((q.bucket, q.window), (Bucket::Minute, Window::Today));
    }

    #[test]
    fn project_sorted_by_cost() {
        let mut a = row(Tool::Pi, "2026-05-01T12:00:00Z", 1, 0.1);
        a.project = "cheap".into();
        let mut b = row(Tool::Pi, "2026-05-01T12:00:00Z", 1, 9.0);
        b.project = "pricey".into();
        let out = by_project(&[a, b]);
        assert_eq!(out[0].0, "pi:pricey");
    }
}
