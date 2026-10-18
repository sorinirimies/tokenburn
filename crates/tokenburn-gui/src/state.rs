//! Application state.

use std::fmt;

use iced::Theme;

use crate::features::theme;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use tokenburn_core::{
    drill_priced, BucketSavings, Collected, Currency, ExternalSavings, Metric, Pie, Query,
    ReportRow, Row, SavedInput, Savings, Selection, Summary, TimeSeries, CURRENCIES,
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

/// Which main panel is shown. The report is home; Settings and About are menu
/// buttons that swap it out and toggle back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Report,
    Settings,
    About,
}

impl View {
    pub const ALL: [View; 3] = [View::Report, View::Settings, View::About];

    /// Whether the view shows usage data (so the totals cards belong above it).
    pub fn is_data(self) -> bool {
        matches!(self, View::Report)
    }

    /// What pressing the menu button for `self` does: open it, or go back to the
    /// report when it is already open.
    pub fn toggled(self, current: View) -> View {
        if self == current {
            View::Report
        } else {
            self
        }
    }
}

impl fmt::Display for View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            View::Report => "Report",
            View::Settings => "Settings",
            View::About => "About",
        })
    }
}

/// The Settings fields' text for `limits`: the amount, or empty when off.
pub fn limit_texts(limits: &tokenburn_core::Limits) -> [String; 3] {
    tokenburn_core::Period::ALL.map(|p| limits.get(p).map(|v| format!("{v}")).unwrap_or_default())
}

pub struct TokenBurn {
    pub query: Query,
    pub view: View,
    /// Index into [`tokenburn_core::THEME_NAMES`].
    pub theme_index: usize,
    /// The theme follows the OS light/dark setting ("System" in the pick list).
    pub follow_system: bool,
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
    /// Savings from rtk and pi compaction, as of the last load.
    pub external: ExternalSavings,
    /// US dollars per million input tokens, to value the savings.
    pub input_price: f64,
    /// Everything saved in the current selection.
    pub savings: Savings,
    /// Daily / weekly / monthly spending limits (information and alarms only).
    pub limits: tokenburn_core::Limits,
    pub limit_statuses: Vec<tokenburn_core::LimitStatus>,
    /// The text in the Settings limit fields (daily, weekly, monthly), US dollars.
    pub limit_inputs: [String; 3],
    /// The latest alert, shown as a banner until dismissed.
    pub alert: Option<String>,
    /// Savings per chart bucket (the same granularity as the bars).
    pub savings_by_bucket: Vec<BucketSavings>,
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
            follow_system: false,
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
            external: ExternalSavings::default(),
            input_price: tokenburn_core::DEFAULT_INPUT_PRICE_PER_MTOK,
            savings: Savings::default(),
            limits: tokenburn_core::Limits::default(),
            limit_statuses: Vec::new(),
            limit_inputs: Default::default(),
            alert: None,
            savings_by_bucket: Vec::new(),
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

    /// Apply preset `index` (clamped; stops following the system) and remember it in
    /// the shared settings, so the TUI and the web dashboard start with it too.
    pub fn set_theme(&mut self, index: usize) {
        self.follow_system = false;
        self.theme_index = index.min(tokenburn_core::THEME_COUNT - 1);
        if self.persist {
            let _ = tokenburn_core::settings::save_theme(self.theme_index);
        }
    }

    /// Follow the OS light/dark setting and remember that.
    pub fn set_follow_system(&mut self) {
        self.follow_system = true;
        self.theme_index = tokenburn_core::current_system_theme_index();
        if self.persist {
            let _ = tokenburn_core::settings::save_system_theme();
        }
    }

    /// Apply a pick-list choice (the System entry or a preset).
    pub fn select_theme(&mut self, choice: theme::ThemeChoice) {
        if choice.is_system() {
            self.set_follow_system();
        } else {
            self.set_theme(choice.index);
        }
    }

    /// Apply a saved/explicit choice without writing it back.
    pub fn apply_theme_choice(&mut self, choice: tokenburn_core::ThemeChoice) {
        self.follow_system = choice == tokenburn_core::ThemeChoice::System;
        self.theme_index = choice.resolve().min(tokenburn_core::THEME_COUNT - 1);
    }

    /// Re-read the OS appearance while following it.
    pub fn refresh_system_theme(&mut self) {
        if self.follow_system {
            self.theme_index = tokenburn_core::current_system_theme_index();
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

    /// Take a fresh limit check: show the progress and raise a banner for new alerts.
    pub fn apply_limit_check(&mut self, check: tokenburn_core::LimitCheck) {
        if let Some(a) = check.alerts.last() {
            self.alert = Some(format!(
                "⚠ {} — {}",
                a.title(),
                a.body(|v| self.currency.format(v))
            ));
        }
        if self.limits != check.limits {
            self.limits = check.limits;
            self.limit_inputs = limit_texts(&self.limits);
        }
        self.limit_statuses = check.statuses;
    }

    /// Read the three limit fields and keep the ones that parse (empty or `0` = off).
    /// Returns `false` (and keeps the old limits) when a field is not a number.
    pub fn apply_limit_inputs(&mut self) -> bool {
        let mut limits = self.limits.clone();
        for (period, text) in tokenburn_core::Period::ALL.iter().zip(&self.limit_inputs) {
            let t = text.trim();
            if t.is_empty() {
                limits.set(*period, None);
            } else {
                match t.parse::<f64>() {
                    Ok(v) if v.is_finite() && v >= 0.0 => limits.set(*period, Some(v)),
                    _ => return false,
                }
            }
        }
        self.limits = limits;
        if self.persist {
            let _ = tokenburn_core::settings::save_limits(&self.limits);
        }
        true
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
        self.external = collected.external;
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
        let d = drill_priced(
            &self.rows,
            &self.query,
            &self.selection,
            &SavedInput {
                price: self.input_price,
                external: Some(&self.external),
            },
        );
        self.selection = d.selection;
        self.report = d.report;
        self.total = d.total;
        self.savings = d.savings;
        self.savings_by_bucket = d.savings_by_bucket;
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
