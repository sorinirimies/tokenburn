use clap::Parser;
use tokenburn_core::{Query, ToolFilter};

/// Desktop GUI for local AI token/cost usage (pi + Zed).
///
/// The flags only choose the initial view; everything can be changed in the window.
#[derive(Debug, Parser, Clone)]
#[command(name = "tokenburn-gui", version, about, long_about = None)]
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
    fn flags_set_initial_view() {
        let q = Args::parse_from(["tokenburn-gui"]).query();
        assert_eq!(q.bucket, Bucket::None);
        let q = Args::parse_from(["tokenburn-gui", "--daily", "--all"]).query();
        assert_eq!((q.bucket, q.window), (Bucket::Day, Window::All));
    }

    #[test]
    fn version_flag_exits_instead_of_opening_a_window() {
        // Regression guard: `--version` / `--help` must never open the window.
        assert!(Args::try_parse_from(["tokenburn-gui", "--version"]).is_err());
    }
}
