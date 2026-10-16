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

    /// Print every supported currency and exit.
    #[arg(long)]
    pub list_currencies: bool,
}

impl Args {
    /// Translate flags into a core [`Query`].
    pub fn query(&self) -> Query {
        Query::from_flags(self.tool, self.hourly, self.daily, self.monthly, self.all)
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
    fn rejects_bad_tool() {
        assert!(Args::try_parse_from(["tokenburn", "--tool", "vim"]).is_err());
    }
}
