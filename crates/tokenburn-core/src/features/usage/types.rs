use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Which tool produced a usage row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tool {
    Pi,
    Zed,
}

impl Tool {
    pub fn as_str(self) -> &'static str {
        match self {
            Tool::Pi => "pi",
            Tool::Zed => "zed",
        }
    }
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One usage record: an assistant turn (pi) or a whole thread (Zed).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub tool: Tool,
    /// pi: project directory name; Zed: model name.
    pub project: String,
    pub id: String,
    pub ts: DateTime<Utc>,
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub cost: f64,
}

/// Aggregated totals over a set of [`Row`]s.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub turns: u64,
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub total_tokens: u64,
    /// Rounded to 4 decimals.
    pub cost_usd: f64,
}

impl Summary {
    /// Summarise `rows`.
    pub fn of<'a>(rows: impl IntoIterator<Item = &'a Row>) -> Self {
        let mut s = Summary::default();
        let mut cost = 0.0;
        for r in rows {
            s.turns += 1;
            s.input += r.input;
            s.output += r.output;
            s.cache_read += r.cache_read;
            s.cache_write += r.cache_write;
            cost += r.cost;
        }
        s.total_tokens = s.input + s.output + s.cache_read + s.cache_write;
        s.cost_usd = (cost * 10_000.0).round() / 10_000.0;
        s
    }
}
