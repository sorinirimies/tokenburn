//! Spending limits: daily, weekly and monthly budgets with warnings and alarms.
//!
//! Limits never block anything — they are information. Each period has an optional
//! cost limit in US dollars ([`Limits`]). Spending reaching `warn_at` of it is a
//! [`Level::Warn`], reaching the limit [`Level::Over`]. Crossing a level fires an
//! [`Alert`] **once per period** (the log lives in `alerts.json` next to the
//! settings, so the CLI watcher, TUI, GUI and web server never repeat each other).
//!
//! Alerts are shown by every front-end and sent as desktop notifications
//! ([`notify`]) on macOS, Linux and Windows.

pub mod ops;
pub mod types;

pub use ops::{
    alert_log_path, check, check_at, collect_rows, notify, pending, run_check, spent, status_line,
    statuses, AlertLog,
};
pub use types::{Alert, Level, LimitCheck, LimitStatus, Limits, Period};
