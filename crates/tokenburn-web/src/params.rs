//! Query-string handling: `?tool=pi&bucket=daily&window=month`.

use tokenburn_core::{Bucket, Metric, Query, ToolFilter, Window};
use topcoat::router::query_params;

/// Window choices offered by the interactive UI. `auto` means "the chosen
/// bucket's natural window" (see [`Bucket::default_window`]).
pub const WINDOW_CHOICES: [&str; 4] = ["auto", "today", "month", "all"];

/// Raw, optional query-string values. Unknown values fall back to defaults
/// instead of failing the request, so a stale bookmark never 400s.
#[query_params(error = redirect("?"))]
pub struct UsageParams {
    pub tool: Option<String>,
    pub bucket: Option<String>,
    pub window: Option<String>,
    /// Colour theme name (see `/api/themes`). Unknown names are ignored.
    pub theme: Option<String>,
    /// Chart metric: `tokens` (default) or `cost`.
    pub metric: Option<String>,
}

impl UsageParams {
    /// Translate into a core [`Query`].
    ///
    /// The window defaults to the chosen bucket's natural window (same rule as
    /// the CLI/TUI/GUI) unless given explicitly.
    pub fn to_query(&self) -> Query {
        parse(
            self.tool.as_deref(),
            self.bucket.as_deref(),
            self.window.as_deref(),
        )
    }
}

/// Parse optional string parameters into a [`Query`], ignoring invalid values.
pub fn parse(tool: Option<&str>, bucket: Option<&str>, window: Option<&str>) -> Query {
    let tool = tool
        .and_then(|s| s.parse::<ToolFilter>().ok())
        .unwrap_or_default();
    let bucket = bucket
        .and_then(|s| s.parse::<Bucket>().ok())
        .unwrap_or_default();
    let window = window
        .and_then(|s| s.parse::<Window>().ok())
        .unwrap_or_else(|| bucket.default_window());
    Query {
        tool,
        bucket,
        window,
    }
}

/// The chart metric from an optional string (unknown values fall back to tokens).
pub fn parse_metric(s: Option<&str>) -> Metric {
    s.and_then(|m| m.parse().ok()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_missing() {
        assert_eq!(parse(None, None, None), Query::default());
    }

    #[test]
    fn bucket_picks_natural_window() {
        let q = parse(None, Some("hourly"), None);
        assert_eq!((q.bucket, q.window), (Bucket::Hour, Window::Today));
        let q = parse(None, Some("monthly"), None);
        assert_eq!(q.window, Window::All);
    }

    #[test]
    fn explicit_window_wins() {
        let q = parse(Some("pi"), Some("daily"), Some("all"));
        assert_eq!(
            (q.tool, q.bucket, q.window),
            (ToolFilter::PI, Bucket::Day, Window::All)
        );
    }

    #[test]
    fn metric_parses_and_defaults_to_tokens() {
        assert_eq!(parse_metric(Some("cost")), Metric::Cost);
        assert_eq!(parse_metric(Some("COST")), Metric::Cost);
        assert_eq!(parse_metric(Some("bytes")), Metric::Tokens);
        assert_eq!(parse_metric(None), Metric::Tokens);
    }

    #[test]
    fn invalid_values_fall_back() {
        // Regression guard: a bad bookmark must not break the page.
        assert_eq!(
            parse(Some("vim"), Some("weekly"), Some("year")),
            Query::default()
        );
    }
}

#[cfg(test)]
mod signal_value_tests {
    use super::*;

    #[test]
    fn every_window_choice_parses_to_a_query() {
        // `auto` is not a real Window: it must fall back to the bucket's natural window.
        let q = parse(None, Some("hourly"), Some("auto"));
        assert_eq!(q.window, Window::Today);
        for w in WINDOW_CHOICES {
            let _ = parse(None, None, Some(w));
        }
    }

    #[test]
    fn hostile_signal_values_are_harmless() {
        // Signals come back from the browser: arbitrary strings must not panic.
        let evil = "'; DROP TABLE <script>\0";
        assert_eq!(parse(Some(evil), Some(evil), Some(evil)), Query::default());
    }
}
