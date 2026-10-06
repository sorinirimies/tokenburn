use clap::Parser;
use tokenburn_core::{Query, ToolFilter};

/// Local AI token/cost usage report for AI coding agents (pi, Zed, Claude Code, Codex, Copilot CLI, Gemini CLI, OpenCode, Amp), read from their session logs on disk.
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

    /// Print JSON instead of a table.
    #[arg(long)]
    pub json: bool,
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
