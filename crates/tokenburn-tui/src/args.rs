use clap::Parser;
use tokenburn_core::{Query, ToolFilter};

/// Interactive terminal UI for local AI token/cost usage (pi, Zed, Claude Code, Codex, …).
///
/// The flags only choose the initial view; everything can be changed live
/// with the keyboard (press `q` to quit).
#[derive(Debug, Parser, Clone)]
#[command(name = "tokenburn-tui", version, about, long_about = None)]
pub struct Args {
    /// Initial source: pi, zed or all.
    #[arg(long, short, value_name = "TOOL", default_value = "all")]
    pub tool: ToolFilter,

    /// Start bucketed by hour (default window: today).
    #[arg(long)]
    pub hourly: bool,

    /// Start bucketed by day (default window: this month).
    #[arg(long)]
    pub daily: bool,

    /// Start bucketed by month (default window: all history).
    #[arg(long)]
    pub monthly: bool,

    /// Start with all logged history.
    #[arg(long, short)]
    pub all: bool,
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
    fn defaults_match_cli() {
        let q = Args::parse_from(["tokenburn-tui"]).query();
        assert_eq!(
            (q.tool, q.bucket, q.window),
            (ToolFilter::All, Bucket::None, Window::Month)
        );
    }

    #[test]
    fn flags_set_initial_view() {
        let q = Args::parse_from(["tokenburn-tui", "--tool", "pi", "--hourly"]).query();
        assert_eq!(
            (q.tool, q.bucket, q.window),
            (ToolFilter::PI, Bucket::Hour, Window::Today)
        );
    }

    #[test]
    fn version_flag_exits_instead_of_starting_the_ui() {
        // Regression guard: `--version` / `--help` must never launch the terminal UI.
        assert!(Args::try_parse_from(["tokenburn-tui", "--version"]).is_err());
        assert!(Args::try_parse_from(["tokenburn-tui", "--help"]).is_err());
    }
}
