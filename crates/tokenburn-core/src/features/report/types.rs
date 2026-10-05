use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Datelike, Local, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::features::usage::{Summary, Tool};

/// Which sources to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolFilter {
    #[default]
    All,
    Pi,
    Zed,
}

impl ToolFilter {
    pub const ALL: [ToolFilter; 3] = [ToolFilter::All, ToolFilter::Pi, ToolFilter::Zed];

    pub fn includes(self, tool: Tool) -> bool {
        matches!(
            (self, tool),
            (ToolFilter::All, _) | (ToolFilter::Pi, Tool::Pi) | (ToolFilter::Zed, Tool::Zed)
        )
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ToolFilter::All => "all",
            ToolFilter::Pi => "pi",
            ToolFilter::Zed => "zed",
        }
    }
}

impl fmt::Display for ToolFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ToolFilter {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "all" => Ok(Self::All),
            "pi" => Ok(Self::Pi),
            "zed" => Ok(Self::Zed),
            other => Err(format!("must be pi, zed, or all (got '{other}')")),
        }
    }
}

/// How rows are grouped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Bucket {
    /// One overall total (+ per-tool breakdown).
    #[default]
    None,
    Hour,
    Day,
    Month,
}

impl Bucket {
    pub const ALL: [Bucket; 4] = [Bucket::None, Bucket::Hour, Bucket::Day, Bucket::Month];

    /// `strftime` format for the bucket label (local time).
    pub fn format(self) -> &'static str {
        match self {
            Bucket::Hour => "%Y-%m-%d %H:00",
            Bucket::Day => "%Y-%m-%d",
            Bucket::Month | Bucket::None => "%Y-%m",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Bucket::None => "total",
            Bucket::Hour => "hourly",
            Bucket::Day => "daily",
            Bucket::Month => "monthly",
        }
    }

    /// Default window for this bucket size.
    pub fn default_window(self) -> Window {
        match self {
            Bucket::Hour => Window::Today,
            Bucket::Month => Window::All,
            Bucket::Day | Bucket::None => Window::Month,
        }
    }
}

impl FromStr for Bucket {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "none" | "total" => Ok(Self::None),
            "hour" | "hourly" => Ok(Self::Hour),
            "day" | "daily" => Ok(Self::Day),
            "month" | "monthly" => Ok(Self::Month),
            other => Err(format!(
                "must be total, hourly, daily or monthly (got '{other}')"
            )),
        }
    }
}

impl fmt::Display for Bucket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// How far back to look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Window {
    Today,
    #[default]
    Month,
    All,
}

impl Window {
    pub const ALL: [Window; 3] = [Window::Today, Window::Month, Window::All];

    pub fn label(self) -> &'static str {
        match self {
            Window::Today => "today",
            Window::Month => "month",
            Window::All => "all",
        }
    }

    /// First instant of the window, in local time, as UTC.
    pub fn start(self, now: DateTime<Local>) -> DateTime<Utc> {
        let local = match self {
            Window::Today => Local
                .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
                .earliest(),
            Window::Month => Local
                .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
                .earliest(),
            Window::All => None,
        };
        local
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
    }
}

impl FromStr for Window {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "today" => Ok(Self::Today),
            "month" => Ok(Self::Month),
            "all" => Ok(Self::All),
            other => Err(format!("must be today, month or all (got '{other}')")),
        }
    }
}

impl fmt::Display for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// A full report request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Query {
    pub tool: ToolFilter,
    pub bucket: Bucket,
    pub window: Window,
}

impl Query {
    /// Build a query from the classic CLI flags. `--hourly` wins over
    /// `--daily` over `--monthly`; `--all` forces the all-history window.
    pub fn from_flags(
        tool: ToolFilter,
        hourly: bool,
        daily: bool,
        monthly: bool,
        all: bool,
    ) -> Self {
        let bucket = if hourly {
            Bucket::Hour
        } else if daily {
            Bucket::Day
        } else if monthly {
            Bucket::Month
        } else {
            Bucket::None
        };
        let window = if all {
            Window::All
        } else {
            bucket.default_window()
        };
        Self {
            tool,
            bucket,
            window,
        }
    }
}

/// One line of a report table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportRow {
    /// Bucket label (`None` for un-bucketed reports).
    pub bucket: Option<String>,
    /// Tool name, or `"TOTAL"`.
    pub tool: String,
    #[serde(flatten)]
    pub summary: Summary,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_round_trips_through_label() {
        for b in Bucket::ALL {
            assert_eq!(b.label().parse::<Bucket>().unwrap(), b);
        }
        assert_eq!("day".parse::<Bucket>().unwrap(), Bucket::Day);
        assert!("weekly".parse::<Bucket>().is_err());
    }

    #[test]
    fn window_round_trips_through_label() {
        for w in Window::ALL {
            assert_eq!(w.label().parse::<Window>().unwrap(), w);
        }
        assert!("year".parse::<Window>().is_err());
    }

    #[test]
    fn tool_filter_parses_case_insensitively() {
        assert_eq!("PI".parse::<ToolFilter>().unwrap(), ToolFilter::Pi);
        assert!("vim".parse::<ToolFilter>().is_err());
    }
}
