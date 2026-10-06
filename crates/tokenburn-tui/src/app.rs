//! Application state.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use crate::features::options::Options;
use crate::features::theme::UiTheme;
use chrono::Local;
use ratatui::widgets::TableState;
use tokenburn_core::{
    build_report, by_project, collect, Bucket, Collected, Query, ReportRow, Row, RowFilter,
    Summary, Tool, ToolFilter, Window,
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
    /// Everything the last load returned (the filter works on this).
    pub all_rows: Vec<Row>,
    /// Rows that pass the filter — what the header, report, chart and pie show.
    pub rows: Vec<Row>,
    /// Free-text + per-tool filter, applied instantly to `all_rows`.
    pub filter: RowFilter,
    /// The `/` text box has the keyboard.
    pub filter_editing: bool,
    /// The `f` tool-checkbox popup is open, and which row is selected.
    pub tool_filter_open: bool,
    pub tool_cursor: usize,
    /// Rows per tool in `all_rows` (for the tool popup).
    pub tool_counts: [usize; 8],
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
            all_rows: Vec::new(),
            rows: Vec::new(),
            filter: RowFilter::default(),
            filter_editing: false,
            tool_filter_open: false,
            tool_cursor: 0,
            tool_counts: [0; 8],
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
                    self.all_rows = collected.rows;
                    self.warnings = collected.warnings;
                    self.is_loading = false;
                    self.last_refresh = Some(Local::now());
                    self.rebuild();
                }
            }
        }
    }

    /// Re-apply the filter to the loaded rows and rebuild everything derived from
    /// them. Instant: no log is re-read.
    pub fn rebuild(&mut self) {
        self.tool_counts = [0; 8];
        for r in &self.all_rows {
            self.tool_counts[r.tool.series_index()] += 1;
        }
        self.rows = self.filter.apply(&self.all_rows);
        self.report = build_report(&self.rows, &self.query);
        self.projects = by_project(&self.rows);
        self.status_message = Some(self.status_text());
        self.clamp_selection();
    }

    /// `updated 12:03:04 · 1,203 of 3,029 rows · auto-refresh 5s`
    fn status_text(&self) -> String {
        let updated = self
            .last_refresh
            .map(|t| t.format("%H:%M:%S").to_string())
            .unwrap_or_else(|| "—".into());
        let count = if self.filter.is_empty() {
            format!("{} rows", self.all_rows.len())
        } else {
            format!("{} of {} rows", self.rows.len(), self.all_rows.len())
        };
        let auto = if self.options.auto_refresh {
            format!(
                " · auto-refresh {}s",
                self.options.refresh_interval().as_secs()
            )
        } else {
            " · auto-refresh off".to_string()
        };
        format!("updated {updated} · {count}{auto}")
    }

    /// Replace the filter text (and re-filter).
    pub fn set_filter_text(&mut self, text: String) {
        self.filter.text = text;
        self.rebuild();
    }

    /// Clear text and tool filters.
    pub fn clear_filter(&mut self) {
        self.filter = RowFilter::default();
        self.rebuild();
    }

    /// Flip the tool at the popup cursor.
    pub fn toggle_tool_at_cursor(&mut self) {
        if let Some(tool) = Tool::ALL.get(self.tool_cursor) {
            self.filter.toggle_tool(*tool);
            self.rebuild();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokenburn_core::Row;

    fn row(project: &str) -> Row {
        Row {
            tool: Tool::Pi,
            project: project.into(),
            id: "s".into(),
            ts: "2026-05-01T12:00:00Z".parse().unwrap(),
            input: 10,
            output: 5,
            cache_read: 0,
            cache_write: 0,
            cost: 1.0,
        }
    }

    #[test]
    fn auto_refresh_waits_for_the_interval() {
        let mut app = App::new();
        app.last_reload = Some(Instant::now());
        app.tick_auto_refresh();
        assert!(!app.is_loading, "not due yet");
    }

    #[test]
    fn auto_refresh_fires_when_the_interval_has_elapsed() {
        let mut app = App::new();
        app.last_reload = Some(Instant::now() - Duration::from_secs(6));
        app.tick_auto_refresh();
        assert!(app.is_loading);
        assert!(app.last_reload.unwrap().elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn auto_refresh_fires_immediately_on_the_first_tick() {
        let mut app = App::new();
        assert!(app.last_reload.is_none());
        app.tick_auto_refresh();
        assert!(app.is_loading);
    }

    #[test]
    fn auto_refresh_respects_the_off_switch_and_in_flight_loads() {
        let mut app = App::new();
        app.options.auto_refresh = false;
        app.tick_auto_refresh();
        assert!(!app.is_loading, "off means off");

        let mut app = App::new();
        app.is_loading = true;
        app.last_reload = Some(Instant::now() - Duration::from_secs(60));
        let before = app.last_reload;
        app.tick_auto_refresh();
        assert_eq!(
            app.last_reload, before,
            "no second load while one is running"
        );
    }

    #[test]
    fn new_data_replaces_the_old_and_stamps_the_time() {
        let mut app = App::new();
        app.bg_tx
            .send(BackgroundResult::Loaded {
                query: app.query,
                collected: Collected {
                    rows: vec![row("a"), row("b")],
                    warnings: vec![],
                },
            })
            .unwrap();
        app.poll_background();
        assert_eq!(app.rows.len(), 2);
        assert!(app.last_refresh.is_some());

        // A later refresh brings a new row: everything derived follows.
        app.bg_tx
            .send(BackgroundResult::Loaded {
                query: app.query,
                collected: Collected {
                    rows: vec![row("a"), row("b"), row("c")],
                    warnings: vec![],
                },
            })
            .unwrap();
        app.poll_background();
        assert_eq!(app.rows.len(), 3);
        assert_eq!(app.projects.len(), 3);
        assert_eq!(app.total().turns, 3);
    }

    #[test]
    fn a_refresh_keeps_the_active_filter() {
        let mut app = App::new();
        app.set_filter_text("a".into());
        app.bg_tx
            .send(BackgroundResult::Loaded {
                query: app.query,
                collected: Collected {
                    rows: vec![row("alpha"), row("beta"), row("gamma")],
                    warnings: vec![],
                },
            })
            .unwrap();
        app.poll_background();
        assert_eq!(app.all_rows.len(), 3);
        assert_eq!(app.rows.len(), 3, "all three contain an 'a'");
        app.set_filter_text("alp".into());
        assert_eq!(app.rows.len(), 1);
        // Another auto-refresh must not reset the filter.
        app.bg_tx
            .send(BackgroundResult::Loaded {
                query: app.query,
                collected: Collected {
                    rows: vec![row("alpha"), row("beta"), row("alpine")],
                    warnings: vec![],
                },
            })
            .unwrap();
        app.poll_background();
        assert_eq!(app.rows.len(), 2);
        assert_eq!(app.filter.text, "alp");
    }

    #[test]
    fn stale_results_for_an_old_query_are_dropped() {
        let mut app = App::new();
        let old = app.query;
        app.query.bucket = Bucket::Day;
        app.bg_tx
            .send(BackgroundResult::Loaded {
                query: old,
                collected: Collected {
                    rows: vec![row("x")],
                    warnings: vec![],
                },
            })
            .unwrap();
        app.poll_background();
        assert!(app.all_rows.is_empty());
    }

    #[test]
    fn the_status_line_reports_filter_counts_and_the_refresh_interval() {
        let mut app = App::new();
        app.all_rows = vec![row("a"), row("b")];
        app.last_refresh = Some(Local::now());
        app.set_filter_text("a".into());
        let s = app.status_message.clone().unwrap();
        assert!(
            s.contains("1 of 2 rows") && s.contains("auto-refresh 5s"),
            "{s}"
        );
        app.options.auto_refresh = false;
        app.rebuild();
        assert!(app.status_message.unwrap().contains("auto-refresh off"));
    }
}
