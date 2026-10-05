//! Application state.

use std::fmt;

use iced::Theme;
use tokenburn_core::{build_report, by_project, Collected, Query, ReportRow, Row, Summary};

/// Which main panel is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Report,
    Projects,
}

impl View {
    pub const ALL: [View; 2] = [View::Report, View::Projects];
}

impl fmt::Display for View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            View::Report => "Report",
            View::Projects => "Projects",
        })
    }
}

pub struct TokenBurn {
    pub query: Query,
    pub view: View,
    pub theme: Theme,
    pub rows: Vec<Row>,
    pub report: Vec<ReportRow>,
    pub projects: Vec<(String, Summary)>,
    pub warnings: Vec<String>,
    pub is_loading: bool,
    pub status_message: Option<String>,
    pub error_message: Option<String>,
    pub tick: u64,
}

impl Default for TokenBurn {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenBurn {
    pub fn new() -> Self {
        Self::with_query(Query::default())
    }

    /// Start from a given query (e.g. from command-line flags).
    pub fn with_query(query: Query) -> Self {
        Self {
            query,
            view: View::default(),
            theme: Theme::TokyoNight,
            rows: Vec::new(),
            report: Vec::new(),
            projects: Vec::new(),
            warnings: Vec::new(),
            is_loading: false,
            status_message: None,
            error_message: None,
            tick: 0,
        }
    }

    pub fn iced_theme(&self) -> Theme {
        self.theme.clone()
    }

    /// Install freshly loaded data.
    pub fn apply(&mut self, collected: Collected) {
        self.report = build_report(&collected.rows, &self.query);
        self.projects = by_project(&collected.rows);
        self.status_message = Some(format!("{} rows loaded", collected.rows.len()));
        self.rows = collected.rows;
        self.warnings = collected.warnings;
        self.error_message = None;
    }

    /// Grand total over every loaded row.
    pub fn total(&self) -> Summary {
        Summary::of(&self.rows)
    }
}
