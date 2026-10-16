use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Which agent harness / IDE produced a usage row.
///
/// Every variant is backed by a collector in `features/<name>/` that reads the
/// logs the tool already keeps on disk (no network). Adding a tool means adding a
/// variant here, a collector module, and one line in [`crate::collect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tool {
    /// [pi](https://github.com/earendil-works/pi-coding-agent)
    Pi,
    /// Zed's agent panel
    Zed,
    /// Claude Code
    Claude,
    /// OpenAI Codex CLI
    Codex,
    /// GitHub Copilot CLI
    Copilot,
    /// Google Gemini CLI
    Gemini,
    /// OpenCode
    OpenCode,
    /// Amp
    Amp,
    /// Cline (VS Code extension / CLI)
    Cline,
    /// Roo Code (VS Code extension)
    Roo,
    /// Kilo Code (VS Code extension)
    Kilo,
    /// Qwen Code
    Qwen,
}

impl Tool {
    /// Every supported tool, in display order.
    /// Number of supported tools (sizes the per-tool arrays).
    pub const COUNT: usize = 12;

    pub const ALL: [Tool; Tool::COUNT] = [
        Tool::Pi,
        Tool::Zed,
        Tool::Claude,
        Tool::Codex,
        Tool::Copilot,
        Tool::Gemini,
        Tool::OpenCode,
        Tool::Amp,
        Tool::Cline,
        Tool::Roo,
        Tool::Kilo,
        Tool::Qwen,
    ];

    /// Short machine name (CLI flag value, JSON, query string).
    pub fn as_str(self) -> &'static str {
        match self {
            Tool::Pi => "pi",
            Tool::Zed => "zed",
            Tool::Claude => "claude",
            Tool::Codex => "codex",
            Tool::Copilot => "copilot",
            Tool::Gemini => "gemini",
            Tool::OpenCode => "opencode",
            Tool::Amp => "amp",
            Tool::Cline => "cline",
            Tool::Roo => "roo",
            Tool::Kilo => "kilo",
            Tool::Qwen => "qwen",
        }
    }

    /// Position in [`Tool::ALL`] — selects the tool's series colour in a theme.
    pub fn series_index(self) -> usize {
        Tool::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }

    /// Human-readable name.
    pub fn display_name(self) -> &'static str {
        match self {
            Tool::Pi => "pi",
            Tool::Zed => "Zed",
            Tool::Claude => "Claude Code",
            Tool::Codex => "Codex CLI",
            Tool::Copilot => "Copilot CLI",
            Tool::Gemini => "Gemini CLI",
            Tool::OpenCode => "OpenCode",
            Tool::Amp => "Amp",
            Tool::Cline => "Cline",
            Tool::Roo => "Roo Code",
            Tool::Kilo => "Kilo Code",
            Tool::Qwen => "Qwen Code",
        }
    }
}

impl std::str::FromStr for Tool {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_ascii_lowercase();
        Tool::ALL
            .into_iter()
            .find(|t| t.as_str() == lower)
            .ok_or_else(|| format!("unknown tool '{s}'"))
    }
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One usage record: an assistant turn, a message, or a whole thread/session,
/// depending on what the tool logs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub tool: Tool,
    /// The project / workspace when the tool logs one (pi, Claude Code, Codex…),
    /// otherwise the model name (Zed, Copilot, Gemini…).
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
