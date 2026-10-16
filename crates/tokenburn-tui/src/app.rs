//! Application state.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use crate::features::options::Options;
use crate::features::theme::UiTheme;
use chrono::Local;
use ratatui::widgets::TableState;
use std::borrow::Cow;
use std::collections::BTreeMap;
use tokenburn_core::features::currency::{index_of, resolve};
use tokenburn_core::{
    build_report, by_project, collect, Bucket, Collected, Query, ReportRow, Row, RowFilter,
    Summary, Tool, ToolFilter, Window,
};
use tokenburn_core::{Currency, CURRENCIES};

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
    /// The theme follows the OS light/dark setting ("System" in the picker).
    pub follow_system: bool,
    /// Last time the OS appearance was polled (while following the system).
    pub last_system_check: Instant,
    /// Display currency (costs are logged in US dollars; this is presentation only).
    pub currency: Currency,
    /// The user's own exchange rates from settings.json (code → units per dollar).
    pub rates: BTreeMap<String, f64>,
    pub currency_picker_open: bool,
    /// When the last reload was started (drives auto-refresh).
    pub last_reload: Option<Instant>,
    /// Everything the last load returned (the filter works on this).
    pub all_rows: Vec<Row>,
    /// Rows that pass the filter — what the header, report, chart and pie show.
    /// How many of `all_rows` pass the filter. (The filtered rows themselves are not kept:
    /// a second copy of every row would double the memory, and nothing needs them once
    /// the report, the projects and `total` are built.)
    pub rows_len: usize,
    /// Totals over the rows that pass the filter.
    total: Summary,
    /// Free-text + per-tool filter, applied instantly to `all_rows`.
    pub filter: RowFilter,
    /// The `/` text box has the keyboard.
    pub filter_editing: bool,
    /// The `f` tool-checkbox popup is open, and which row is selected.
    pub tool_filter_open: bool,
    pub tool_cursor: usize,
    /// Rows per tool in `all_rows` (for the tool popup).
    pub tool_counts: [usize; Tool::COUNT],
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
            follow_system: false,
            last_system_check: Instant::now(),
            currency: Currency::USD,
            rates: BTreeMap::new(),
            currency_picker_open: false,
            last_reload: None,
            all_rows: Vec::new(),
            rows_len: 0,
            total: Summary::default(),
            filter: RowFilter::default(),
            filter_editing: false,
            tool_filter_open: false,
            tool_cursor: 0,
            tool_counts: [0; Tool::COUNT],
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

    /// How long the event loop may sleep before it must look again: a frame
    /// (~30 fps) while a load animates the spinner, otherwise until the next
    /// auto-refresh is due (capped, so a finished load shows up promptly).
    pub fn poll_timeout(&self) -> std::time::Duration {
        use std::time::Duration;
        const FRAME: Duration = Duration::from_millis(33);
        const IDLE: Duration = Duration::from_millis(250);
        if self.is_loading {
            return FRAME;
        }
        if !self.options.auto_refresh {
            return IDLE;
        }
        let due_in = match self.last_reload {
            None => Duration::ZERO,
            Some(t) => self.options.refresh_interval().saturating_sub(t.elapsed()),
        };
        due_in.clamp(FRAME, IDLE)
    }

    /// Drain finished background work. Results for a stale query are dropped.
    /// Returns `true` when new data arrived (the screen needs a redraw).
    pub fn poll_background(&mut self) -> bool {
        let mut changed = false;
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
                    changed = true;
                }
            }
        }
        changed
    }

    /// Re-apply the filter to the loaded rows and rebuild everything derived from
    /// them. Instant: no log is re-read.
    pub fn rebuild(&mut self) {
        self.tool_counts = [0; Tool::COUNT];
        for r in &self.all_rows {
            self.tool_counts[r.tool.series_index()] += 1;
        }
        // With no filter every row passes, so borrow instead of copying them all.
        let filtered: Cow<[Row]> = if self.filter.is_empty() {
            Cow::Borrowed(&self.all_rows)
        } else {
            Cow::Owned(self.filter.apply(&self.all_rows))
        };
        self.rows_len = filtered.len();
        self.total = Summary::of(filtered.iter());
        self.report = build_report(&filtered, &self.query);
        self.projects = by_project(&filtered);
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
            format!("{} of {} rows", self.rows_len, self.all_rows.len())
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

    fn apply_theme_index(&mut self, index: usize) {
        self.theme_index = index.min(tokenburn_core::THEME_COUNT - 1);
        self.theme = UiTheme::by_index(self.theme_index);
    }

    /// Apply the preset at `index` (clamps; stops following the system) and remember
    /// it in the shared settings so the GUI and web dashboard start with it too.
    pub fn set_theme(&mut self, index: usize) {
        self.follow_system = false;
        self.apply_theme_index(index);
        self.persist_theme();
    }

    /// Follow the OS light/dark setting and remember that.
    pub fn set_follow_system(&mut self) {
        self.follow_system = true;
        self.apply_theme_index(tokenburn_core::current_system_theme_index());
        self.persist_theme();
    }

    /// Position of the active choice in the picker (`0` = System).
    pub fn picker_index(&self) -> usize {
        if self.follow_system {
            0
        } else {
            self.theme_index + 1
        }
    }

    /// Select picker row `i` (`0` = System, `n + 1` = preset `n`; clamps).
    pub fn select_picker(&mut self, i: usize) {
        let i = i.min(crate::features::theme::PICKER_LEN - 1);
        if i == 0 {
            self.set_follow_system();
        } else {
            self.set_theme(i - 1);
        }
    }

    /// Move `delta` rows through the picker (negative: back), wrapping around.
    pub fn cycle_theme(&mut self, delta: isize) {
        let n = crate::features::theme::PICKER_LEN as isize;
        let next = (self.picker_index() as isize + delta).rem_euclid(n) as usize;
        self.select_picker(next);
    }

    /// Apply a saved/explicit choice without writing it back.
    pub fn apply_theme_choice(&mut self, choice: tokenburn_core::ThemeChoice) {
        self.follow_system = choice == tokenburn_core::ThemeChoice::System;
        self.apply_theme_index(choice.resolve());
    }

    /// Re-read the OS appearance (throttled to every 3 s) while following the system.
    /// Returns `true` when the theme changed (needs a redraw).
    pub fn refresh_system_theme(&mut self) -> bool {
        if !self.follow_system || self.last_system_check.elapsed().as_secs() < 3 {
            return false;
        }
        self.last_system_check = Instant::now();
        let index = tokenburn_core::current_system_theme_index();
        let changed = index != self.theme_index;
        self.apply_theme_index(index);
        changed
    }

    /// Show costs in the currency at `index` of [`CURRENCIES`] (clamped), with the
    /// user's own rate if they set one, and remember the choice for the other front-ends.
    pub fn set_currency(&mut self, index: usize) {
        let code = CURRENCIES[index.min(CURRENCIES.len() - 1)].code;
        self.currency = resolve(code, &self.rates);
        if self.persist {
            let _ = tokenburn_core::settings::save_currency(code);
        }
    }

    /// Move `delta` currencies forward (negative: back), wrapping around.
    pub fn cycle_currency(&mut self, delta: isize) {
        let n = CURRENCIES.len() as isize;
        let at = index_of(self.currency.code) as isize;
        self.set_currency((at + delta).rem_euclid(n) as usize);
    }

    /// Best-effort save; tests opt out so they never touch the user's real config.
    fn persist_theme(&self) {
        if self.persist {
            let _ = if self.follow_system {
                tokenburn_core::settings::save_system_theme()
            } else {
                tokenburn_core::settings::save_theme(self.theme_index)
            };
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

    /// Grand total over the rows that pass the filter.
    pub fn total(&self) -> Summary {
        self.total
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
        assert_eq!(app.rows_len, 2);
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
        assert_eq!(app.rows_len, 3);
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
        assert_eq!(app.rows_len, 3, "all three contain an 'a'");
        app.set_filter_text("alp".into());
        assert_eq!(app.rows_len, 1);
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
        assert_eq!(app.rows_len, 2);
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
    fn poll_background_reports_whether_anything_arrived() {
        let mut app = App::new();
        assert!(!app.poll_background(), "nothing queued → no redraw needed");
        app.bg_tx
            .send(BackgroundResult::Loaded {
                query: app.query,
                collected: Collected::default(),
            })
            .unwrap();
        assert!(app.poll_background());
        assert!(!app.poll_background());
    }

    #[test]
    fn the_event_loop_sleeps_longer_when_idle() {
        let mut app = App::new();
        app.is_loading = true;
        assert_eq!(
            app.poll_timeout(),
            Duration::from_millis(33),
            "animate while loading"
        );

        app.is_loading = false;
        app.last_reload = Some(Instant::now());
        let idle = app.poll_timeout();
        assert!(
            idle > Duration::from_millis(33) && idle <= Duration::from_millis(250),
            "{idle:?}"
        );

        app.last_reload = Some(Instant::now() - Duration::from_secs(60));
        assert_eq!(
            app.poll_timeout(),
            Duration::from_millis(33),
            "a refresh is due: wake up now"
        );

        app.options.auto_refresh = false;
        assert_eq!(
            app.poll_timeout(),
            Duration::from_millis(250),
            "no refresh scheduled: sleep"
        );
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
