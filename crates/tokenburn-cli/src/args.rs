use clap::Parser;
use tokenburn_core::{Currency, Query, ToolFilter};

/// Local AI token/cost usage report for AI coding agents (pi, Zed, Claude Code, Codex, Copilot CLI, Gemini CLI, OpenCode, Amp, Cline, Roo Code, Kilo Code, Qwen Code), read from their session logs on disk.
///
/// Reads local logs only — no network calls.
#[derive(Debug, Parser, Clone)]
#[command(name = "tokenburn", version, about, long_about = None)]
pub struct Args {
    /// Source to read: all, pi, zed, claude, codex, copilot, gemini, opencode or amp.
    #[arg(long, short, value_name = "TOOL", default_value = "all")]
    pub tool: ToolFilter,

    /// Bucket by minute (default window: today).
    #[arg(long)]
    pub minutely: bool,

    /// Bucket by hour (default window: today).
    #[arg(long)]
    pub hourly: bool,

    /// Bucket by day (default window: this month).
    #[arg(long)]
    pub daily: bool,

    /// Bucket by month (default window: all logged history).
    #[arg(long)]
    pub monthly: bool,

    /// Ignore the default window, use all logged history.
    #[arg(long, short)]
    pub all: bool,

    /// Print JSON instead of a table. JSON costs are always US dollars (`cost_usd`).
    #[arg(long)]
    pub json: bool,

    /// Show costs in this currency (e.g. EUR, GBP, JPY). Defaults to the one saved by
    /// the other front-ends (USD if none). Rates are fixed estimates: set exact ones under
    /// "rates" in settings.json.
    #[arg(long, short = 'c', value_name = "CODE")]
    pub currency: Option<Currency>,

    /// Also show what you saved: prompt-cache hits, rtk output filtering and pi
    /// context compaction, in tokens and money. With `--json` the output becomes
    /// `{"report": [...], "savings": {...}}`.
    #[arg(long, short = 's')]
    pub savings: bool,

    /// US dollars per million input tokens, used to value the savings (default:
    /// `input_price_per_mtok` from settings.json, else 3.0).
    #[arg(long, value_name = "USD", value_parser = parse_price)]
    pub input_price: Option<f64>,

    /// Show your daily / weekly / monthly spending limits and how much of each is used.
    #[arg(long)]
    pub limits: bool,

    /// Set the daily spending limit in US dollars (0 clears it). Limits only inform
    /// and alarm — nothing is ever blocked.
    #[arg(long, value_name = "USD", value_parser = parse_limit)]
    pub limit_daily: Option<f64>,

    /// Set the weekly (Monday start) spending limit in US dollars (0 clears it).
    #[arg(long, value_name = "USD", value_parser = parse_limit)]
    pub limit_weekly: Option<f64>,

    /// Set the monthly spending limit in US dollars (0 clears it).
    #[arg(long, value_name = "USD", value_parser = parse_limit)]
    pub limit_monthly: Option<f64>,

    /// Keep running and raise a desktop notification when a limit is nearly
    /// reached or exceeded, even with no TokenBurn window open. Optional check
    /// interval in seconds (default 60).
    #[arg(long, value_name = "SECS", num_args = 0..=1, default_missing_value = "60", value_parser = parse_secs)]
    pub watch: Option<u64>,

    /// Print every supported currency and exit.
    #[arg(long)]
    pub list_currencies: bool,
}

fn parse_limit(s: &str) -> Result<f64, String> {
    s.parse::<f64>()
        .ok()
        .filter(|p| p.is_finite() && *p >= 0.0)
        .ok_or_else(|| format!("must be zero or a positive number (got '{s}')"))
}

fn parse_secs(s: &str) -> Result<u64, String> {
    s.parse::<u64>()
        .ok()
        .filter(|n| *n >= 5)
        .ok_or_else(|| format!("must be at least 5 seconds (got '{s}')"))
}

fn parse_price(s: &str) -> Result<f64, String> {
    s.parse::<f64>()
        .ok()
        .filter(|p| p.is_finite() && *p > 0.0)
        .ok_or_else(|| format!("must be a positive number (got '{s}')"))
}

impl Args {
    /// Translate flags into a core [`Query`].
    pub fn query(&self) -> Query {
        Query::from_flags(
            self.tool,
            self.minutely,
            self.hourly,
            self.daily,
            self.monthly,
            self.all,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokenburn_core::{Bucket, Window};

    #[test]
    fn defaults() {
        let a = Args::parse_from(["tokenburn"]);
        let q = a.query();
        assert_eq!(
            (q.tool, q.bucket, q.window),
            (ToolFilter::All, Bucket::None, Window::Month)
        );
    }

    #[test]
    fn currency_flags() {
        let a = Args::parse_from(["tokenburn", "--currency", "eur"]);
        assert_eq!(a.currency.unwrap().code, "EUR");
        assert!(Args::parse_from(["tokenburn"]).currency.is_none());
        assert!(Args::parse_from(["tokenburn", "--list-currencies"]).list_currencies);
        assert!(Args::try_parse_from(["tokenburn", "-c", "xyz"]).is_err());
    }

    #[test]
    fn flags() {
        let a = Args::parse_from(["tokenburn", "--tool", "pi", "--daily", "--all"]);
        let q = a.query();
        assert_eq!(
            (q.tool, q.bucket, q.window),
            (ToolFilter::PI, Bucket::Day, Window::All)
        );
    }

    #[test]
    fn savings_flags() {
        let a = Args::parse_from(["tokenburn"]);
        assert!(!a.savings && a.input_price.is_none());
        let a = Args::parse_from(["tokenburn", "-s", "--input-price", "15"]);
        assert!(a.savings);
        assert_eq!(a.input_price, Some(15.0));
        for bad in ["0", "-1", "abc", "inf"] {
            assert!(
                Args::try_parse_from(["tokenburn", "--input-price", bad]).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn limit_flags() {
        let a = Args::parse_from(["tokenburn", "--limit-daily", "20", "--limit-monthly", "0"]);
        assert_eq!(
            (a.limit_daily, a.limit_weekly, a.limit_monthly),
            (Some(20.0), None, Some(0.0))
        );
        assert!(Args::try_parse_from(["tokenburn", "--limit-daily", "-1"]).is_err());
        assert!(Args::try_parse_from(["tokenburn", "--limit-daily", "x"]).is_err());
        assert_eq!(Args::parse_from(["tokenburn", "--watch"]).watch, Some(60));
        assert_eq!(
            Args::parse_from(["tokenburn", "--watch", "30"]).watch,
            Some(30)
        );
        assert!(Args::try_parse_from(["tokenburn", "--watch", "1"]).is_err());
        assert!(Args::parse_from(["tokenburn", "--limits"]).limits);
        assert!(Args::parse_from(["tokenburn"]).watch.is_none());
    }

    #[test]
    fn rejects_bad_tool() {
        assert!(Args::try_parse_from(["tokenburn", "--tool", "vim"]).is_err());
    }
}
