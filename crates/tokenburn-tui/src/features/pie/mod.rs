//! Share pie chart ([`tui_piechart`]): who burned the most?

pub mod view;

use tokenburn_core::Summary;

/// What a pie slice measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    /// Estimated `$` cost (pi reports it).
    Cost,
    /// Total tokens — used when no cost is known (e.g. Zed only).
    Tokens,
}

impl Metric {
    pub fn label(self) -> &'static str {
        match self {
            Metric::Cost => "cost",
            Metric::Tokens => "tokens",
        }
    }
}

/// Turn the per-project breakdown into at most `top_n` slices plus an
/// `other` slice for the remainder.
///
/// The metric is cost when any entry has a cost, tokens otherwise. Zero-valued
/// entries are dropped. Input is expected sorted most-expensive first (as
/// [`tokenburn_core::by_project`] returns it) but is re-sorted by the chosen
/// metric to stay correct for tokens.
pub fn pie_data(projects: &[(String, Summary)], top_n: usize) -> (Vec<(String, f64)>, Metric) {
    let metric = if projects.iter().any(|(_, s)| s.cost_usd > 0.0) {
        Metric::Cost
    } else {
        Metric::Tokens
    };
    let value = |s: &Summary| match metric {
        Metric::Cost => s.cost_usd,
        Metric::Tokens => s.total_tokens as f64,
    };

    let mut items: Vec<(String, f64)> = projects
        .iter()
        .map(|(n, s)| (short_name(n), value(s)))
        .filter(|(_, v)| *v > 0.0)
        .collect();
    items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let top_n = top_n.max(1);
    if items.len() > top_n {
        let rest: f64 = items.split_off(top_n).iter().map(|(_, v)| v).sum();
        items.push(("other".to_string(), rest));
    }
    (items, metric)
}

/// `pi:Users-me-Projects-tokenburn` → `pi:tokenburn` (last path-ish segment).
pub fn short_name(full: &str) -> String {
    match full.split_once(':') {
        Some((tool, rest)) => {
            let last = rest
                .rsplit(['-', '/', '\\'])
                .find(|s| !s.is_empty())
                .unwrap_or(rest);
            format!("{tool}:{last}")
        }
        None => full.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(cost: f64, tokens: u64) -> Summary {
        Summary {
            turns: 1,
            total_tokens: tokens,
            cost_usd: cost,
            ..Summary::default()
        }
    }

    #[test]
    fn cost_metric_when_any_cost() {
        let (d, m) = pie_data(
            &[("pi:a".into(), s(2.0, 10)), ("zed:m".into(), s(0.0, 99))],
            6,
        );
        assert_eq!(m, Metric::Cost);
        assert_eq!(d.len(), 1, "zero-cost entries are dropped");
    }

    #[test]
    fn token_metric_when_no_cost_anywhere() {
        let (d, m) = pie_data(
            &[("zed:m".into(), s(0.0, 50)), ("zed:n".into(), s(0.0, 70))],
            6,
        );
        assert_eq!(m, Metric::Tokens);
        assert_eq!(d[0].0, "zed:n", "sorted by tokens");
    }

    #[test]
    fn folds_the_tail_into_other() {
        let p: Vec<_> = (1..=5)
            .map(|i| (format!("pi:p{i}"), s(i as f64, 1)))
            .collect();
        let (d, _) = pie_data(&p, 3);
        assert_eq!(d.len(), 4);
        assert_eq!(d[3].0, "other");
        assert!((d[3].1 - 3.0).abs() < 1e-9, "1+2");
    }

    #[test]
    fn empty_input_is_empty() {
        assert!(pie_data(&[], 6).0.is_empty());
    }

    #[test]
    fn short_names_keep_tool_and_last_segment() {
        assert_eq!(short_name("pi:Users-me-Projects-tokenburn"), "pi:tokenburn");
        assert_eq!(short_name("zed:claude-sonnet"), "zed:sonnet");
        assert_eq!(short_name("plain"), "plain");
    }
}
