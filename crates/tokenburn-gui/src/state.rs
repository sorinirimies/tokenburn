//! Application state.

use std::fmt;

use iced::Theme;

use crate::features::theme;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use tokenburn_core::{
    drill, Collected, Currency, Metric, Pie, Query, ReportRow, Row, Selection, Summary, TimeSeries,
    CURRENCIES,
};

/// How often the GUI re-reads the logs by itself.
pub const AUTO_REFRESH: std::time::Duration = std::time::Duration::from_secs(10);

/// How long the charts take to grow in after a (re)load or a metric change.
pub const ENTRY_ANIMATION: Duration = Duration::from_millis(700);
/// How long a selection takes to settle (the slice pops out, the rest fade).
pub const SELECT_ANIMATION: Duration = Duration::from_millis(220);

/// Fast start, gentle stop.
pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// Progress of an animation that began at `start`, given `now` (1.0 when idle).
fn progress(start: Option<Instant>, now: Instant, len: Duration) -> f32 {
    start.map_or(1.0, |s| {
        (now.saturating_duration_since(s).as_secs_f32() / len.as_secs_f32()).min(1.0)
    })
}

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
    /// Display currency (costs are logged in US dollars; this is presentation only).
    pub currency: Currency,
    /// The user's own exchange rates from settings.json (code → units per dollar).
    pub rates: BTreeMap<String, f64>,
    /// One-time platform setup (the macOS Window menu) has been requested.
    pub platform_ready: bool,
    /// Write settings (theme) to disk. `false` in tests and `TokenBurn::new()`.
    pub persist: bool,
    pub rows: Vec<Row>,
    pub report: Vec<ReportRow>,
    pub projects: Vec<(String, Summary)>,
    /// Stacked per-tool totals over time (the bar chart).
    pub series: TimeSeries,
    /// Share by project (the donut).
    pub pie: Pie,
    /// What the charts measure.
    pub chart_metric: Metric,
    /// The clicked bar and/or donut slice; everything else is scoped to it.
    pub selection: Selection,
    /// Headline totals for the current selection.
    pub total: Summary,
    /// Entry animation of the charts, eased 0 → 1 (1 = finished).
    pub anim: f32,
    anim_started: Option<Instant>,
    /// Selection highlight animation, eased 0 → 1.
    pub sel_anim: f32,
    sel_started: Option<Instant>,
    /// `(query, metric)` the charts last animated in for; refreshes of the same
    /// view must not replay the animation every few seconds.
    anim_key: Option<(Query, Metric)>,
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
            currency: Currency::USD,
            rates: BTreeMap::new(),
            platform_ready: false,
            persist: false,
            rows: Vec::new(),
            report: Vec::new(),
            projects: Vec::new(),
            series: TimeSeries::default(),
            pie: Pie {
                metric: Metric::Tokens,
                slices: Vec::new(),
            },
            chart_metric: Metric::Tokens,
            selection: Selection::default(),
            total: Summary::default(),
            anim: 1.0,
            anim_started: None,
            sel_anim: 1.0,
            sel_started: None,
            anim_key: None,
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

    /// Show costs in the currency at `index` of [`CURRENCIES`], with the user's own
    /// rate if they set one, and remember the choice for the other front-ends.
    pub fn set_currency(&mut self, index: usize) {
        let code = CURRENCIES[index.min(CURRENCIES.len() - 1)].code;
        self.currency = tokenburn_core::features::currency::resolve(code, &self.rates);
        if self.persist {
            let _ = tokenburn_core::settings::save_currency(code);
        }
    }

    /// Install freshly loaded data.
    pub fn apply(&mut self, collected: Collected) {
        self.last_updated = Some(chrono::Local::now());
        self.status_message = Some(format!(
            "Updated {} · {} rows · auto-refresh every {} s",
            chrono::Local::now().format("%H:%M:%S"),
            collected.rows.len(),
            AUTO_REFRESH.as_secs()
        ));
        self.rows = collected.rows;
        self.warnings = collected.warnings;
        self.error_message = None;
        self.recompute();
        // Grow the charts in the first time a view appears — not on every refresh.
        let key = (self.query, self.chart_metric);
        if self.anim_key != Some(key) {
            self.anim_key = Some(key);
            self.restart_entry_animation();
        }
    }

    /// Rebuild the report, totals and both charts for the current selection.
    pub fn recompute(&mut self) {
        let d = drill(&self.rows, &self.query, &self.selection);
        self.selection = d.selection;
        self.report = d.report;
        self.total = d.total;
        self.projects = d.projects;
        self.series = d.series;
        self.pie = d.pie;
    }

    /// A bar was clicked: scope everything to that bucket (click again to clear).
    pub fn click_bucket(&mut self, label: &str) {
        self.selection.toggle_bucket(label);
        self.selection_changed();
    }

    /// A donut slice (or its legend row) was clicked: scope to that project.
    pub fn click_project(&mut self, key: &str) {
        self.selection.toggle_project(key);
        self.selection_changed();
    }

    pub fn clear_selection(&mut self) {
        self.selection.clear();
        self.selection_changed();
    }

    fn selection_changed(&mut self) {
        self.recompute();
        self.sel_anim = 0.0;
        self.sel_started = Some(Instant::now());
    }

    pub fn restart_entry_animation(&mut self) {
        self.anim = 0.0;
        self.anim_started = Some(Instant::now());
    }

    /// Whether a frame subscription is needed.
    pub fn is_animating(&self) -> bool {
        self.anim_started.is_some() || self.sel_started.is_some()
    }

    /// Advance the animations to `now`; finished ones switch themselves off.
    pub fn tick_animation(&mut self, now: Instant) {
        let p = progress(self.anim_started, now, ENTRY_ANIMATION);
        self.anim = ease_out(p);
        if p >= 1.0 {
            self.anim_started = None;
        }
        let p = progress(self.sel_started, now, SELECT_ANIMATION);
        self.sel_anim = ease_out(p);
        if p >= 1.0 {
            self.sel_started = None;
        }
    }

    /// Grand total for the current selection.
    pub fn total(&self) -> Summary {
        self.total
    }
}
