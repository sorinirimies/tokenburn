//! Application state.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use crate::features::options::Options;
use crate::features::theme::UiTheme;
use chrono::Local;
use ratatui::widgets::TableState;
use tokenburn_core::{
    build_report, by_project, collect, Bucket, Collected, Query, ReportRow, Row, Summary,
    ToolFilter, Window,
};

/// Which main panel is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    /// Aggregated report table + chart.
    #[default]
    Report,
    /// Per-project / per-model breakdown.
    Projects,
}

/// Messages sent from the loader thread.
pub enum BackgroundResult {
    Loaded { query: Query, collected: Collected },
}

pub struct App {
    pub query: Query,
    pub view: View,
    pub options: Options,
    /// Index into [`tokenburn_core::THEME_NAMES`].
    pub theme_index: usize,
    /// Colours of the active theme (what every view reads).
    pub theme: UiTheme,
    pub theme_picker_open: bool,
    /// When the last reload was started (drives auto-refresh).
    pub last_reload: Option<Instant>,
    pub rows: Vec<Row>,
    pub report: Vec<ReportRow>,
    pub projects: Vec<(String, Summary)>,
    pub warnings: Vec<String>,
    pub table_state: TableState,
    pub is_loading: bool,
    pub status_message: Option<String>,
    pub last_refresh: Option<chrono::DateTime<Local>>,
    pub tick_count: u64,
    pub should_quit: bool,
    /// Write settings (theme) to disk. `false` in tests.
    pub persist: bool,
    pub bg_tx: Sender<BackgroundResult>,
    bg_rx: Receiver<BackgroundResult>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let (bg_tx, bg_rx) = mpsc::channel();
        Self {
            query: Query::default(),
            view: View::default(),
            options: Options::default(),
            theme_index: 0,
            theme: UiTheme::by_index(0),
            theme_picker_open: false,
            last_reload: None,
            rows: Vec::new(),
            report: Vec::new(),
            projects: Vec::new(),
            warnings: Vec::new(),
            table_state: TableState::default(),
            is_loading: false,
            status_message: None,
            last_refresh: None,
            tick_count: 0,
            should_quit: false,
            persist: false,
            bg_tx,
            bg_rx,
        }
    }

    /// Re-read the logs in a background thread for the current query.
    pub fn reload(&mut self) {
        self.is_loading = true;
        self.last_reload = Some(Instant::now());
        let query = self.query;
        let tx = self.bg_tx.clone();
        thread::spawn(move || {
            let start = query.window.start(Local::now());
            let collected = collect(query.tool, start);
            let _ = tx.send(BackgroundResult::Loaded { query, collected });
        });
    }

    /// Start a reload when auto-refresh is on and the interval has elapsed.
    pub fn tick_auto_refresh(&mut self) {
        if !self.options.auto_refresh || self.is_loading {
            return;
        }
        let due = self
            .last_reload
            .is_none_or(|t| t.elapsed() >= self.options.refresh_interval());
        if due {
            self.reload();
        }
    }

    /// Drain finished background work. Results for a stale query are dropped.
    pub fn poll_background(&mut self) {
        while let Ok(msg) = self.bg_rx.try_recv() {
            match msg {
                BackgroundResult::Loaded { query, collected } => {
                    if query != self.query {
                        continue;
                    }
                    self.report = build_report(&collected.rows, &query);
                    self.projects = by_project(&collected.rows);
                    self.rows = collected.rows;
                    self.warnings = collected.warnings;
                    self.is_loading = false;
                    self.last_refresh = Some(Local::now());
                    self.status_message = Some(format!("{} rows loaded", self.rows.len()));
                    self.clamp_selection();
                }
            }
        }
    }

    /// Number of selectable lines in the current view.
    pub fn line_count(&self) -> usize {
        match self.view {
            View::Report => self.report.len(),
            View::Projects => self.projects.len(),
        }
    }

    pub fn clamp_selection(&mut self) {
        let n = self.line_count();
        if n == 0 {
            self.table_state.select(None);
        } else {
            let i = self.table_state.selected().unwrap_or(0).min(n - 1);
            self.table_state.select(Some(i));
        }
    }

    /// Switch bucket size; also resets to that bucket's natural window.
    pub fn set_bucket(&mut self, bucket: Bucket) {
        if self.query.bucket == bucket {
            return;
        }
        self.query.bucket = bucket;
        self.query.window = bucket.default_window();
        self.reload();
    }

    pub fn cycle_bucket(&mut self, forward: bool) {
        let all = Bucket::ALL;
        let i = all
            .iter()
            .position(|b| *b == self.query.bucket)
            .unwrap_or(0);
        let n = all.len();
        let next = if forward {
            (i + 1) % n
        } else {
            (i + n - 1) % n
        };
        self.set_bucket(all[next]);
    }

    pub fn cycle_tool(&mut self) {
        let all = ToolFilter::ALL;
        let i = all.iter().position(|t| *t == self.query.tool).unwrap_or(0);
        self.query.tool = all[(i + 1) % all.len()];
        self.reload();
    }

    pub fn cycle_window(&mut self) {
        self.query.window = match self.query.window {
            Window::Today => Window::Month,
            Window::Month => Window::All,
            Window::All => Window::Today,
        };
        self.reload();
    }

    /// Apply the theme at `index` (wraps / clamps) and remember it in the shared
    /// settings so the GUI and web dashboard start with it too.
    pub fn set_theme(&mut self, index: usize) {
        self.theme_index = index.min(tokenburn_core::THEME_COUNT - 1);
        self.theme = UiTheme::by_index(self.theme_index);
        self.persist_theme();
    }

    /// Move `delta` themes forward (negative: back), wrapping around.
    pub fn cycle_theme(&mut self, delta: isize) {
        let n = tokenburn_core::THEME_COUNT as isize;
        let next = (self.theme_index as isize + delta).rem_euclid(n) as usize;
        self.set_theme(next);
    }

    /// Best-effort save; tests opt out so they never touch the user's real config.
    fn persist_theme(&self) {
        if self.persist {
            let _ = tokenburn_core::settings::save_theme(self.theme_index);
        }
    }

    pub fn toggle_view(&mut self) {
        self.view = match self.view {
            View::Report => View::Projects,
            View::Projects => View::Report,
        };
        self.table_state.select(None);
        self.clamp_selection();
    }

    /// Grand total over every loaded row.
    pub fn total(&self) -> Summary {
        Summary::of(&self.rows)
    }
}
