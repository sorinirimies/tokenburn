//! Application state.

use std::fmt;

use iced::Theme;

use crate::features::theme;
use tokenburn_core::{build_report, by_project, Collected, Query, ReportRow, Row, Summary};

/// How often the GUI re-reads the logs by itself.
pub const AUTO_REFRESH: std::time::Duration = std::time::Duration::from_secs(10);

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
    /// Index into [`tokenburn_core::THEME_NAMES`].
    pub theme_index: usize,
    /// Write settings (theme) to disk. `false` in tests and `TokenBurn::new()`.
    pub persist: bool,
    pub rows: Vec<Row>,
    pub report: Vec<ReportRow>,
    pub projects: Vec<(String, Summary)>,
    pub warnings: Vec<String>,
    pub is_loading: bool,
    pub status_message: Option<String>,
    pub error_message: Option<String>,
    pub tick: u64,
    /// When the data on screen was last refreshed.
    pub last_updated: Option<chrono::DateTime<chrono::Local>>,
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
            theme_index: 0,
            persist: false,
            rows: Vec::new(),
            report: Vec::new(),
            projects: Vec::new(),
            warnings: Vec::new(),
            is_loading: false,
            status_message: None,
            error_message: None,
            tick: 0,
            last_updated: None,
        }
    }

    pub fn iced_theme(&self) -> Theme {
        theme::iced_theme(self.theme_index)
    }

    /// Apply a theme and remember it in the shared settings, so the TUI and the
    /// web dashboard start with it too.
    pub fn set_theme(&mut self, index: usize) {
        self.theme_index = index.min(tokenburn_core::THEME_COUNT - 1);
        if self.persist {
            let _ = tokenburn_core::settings::save_theme(self.theme_index);
        }
    }

    /// Install freshly loaded data.
    pub fn apply(&mut self, collected: Collected) {
        self.last_updated = Some(chrono::Local::now());
        self.report = build_report(&collected.rows, &self.query);
        self.projects = by_project(&collected.rows);
        self.status_message = Some(format!(
            "Updated {} · {} rows · auto-refresh every {} s",
            chrono::Local::now().format("%H:%M:%S"),
            collected.rows.len(),
            AUTO_REFRESH.as_secs()
        ));
        self.rows = collected.rows;
        self.warnings = collected.warnings;
        self.error_message = None;
    }

    /// Grand total over every loaded row.
    pub fn total(&self) -> Summary {
        Summary::of(&self.rows)
    }
}
