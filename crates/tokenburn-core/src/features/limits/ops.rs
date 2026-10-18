use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, Duration, Local, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use super::types::{Alert, Level, LimitCheck, LimitStatus, Limits, Period};
use crate::features::report::ToolFilter;
use crate::features::usage::Row;

impl Period {
    /// First instant of the period containing `now`, in local time, as UTC.
    pub fn start(self, now: DateTime<Local>) -> DateTime<Utc> {
        let date = match self {
            Period::Day => now.date_naive(),
            Period::Week => {
                now.date_naive() - Duration::days(now.weekday().num_days_from_monday() as i64)
            }
            Period::Month => now.date_naive().with_day(1).unwrap_or(now.date_naive()),
        };
        Local
            .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap_or_default())
            .earliest()
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
    }
}

/// US dollars spent in the period containing `now`.
pub fn spent(rows: &[Row], period: Period, now: DateTime<Local>) -> f64 {
    let start = period.start(now);
    rows.iter().filter(|r| r.ts >= start).map(|r| r.cost).sum()
}

fn level(spent: f64, limit: f64, warn_at: f64) -> Level {
    if spent >= limit {
        Level::Over
    } else if spent >= limit * warn_at {
        Level::Warn
    } else {
        Level::Ok
    }
}

/// The status of every period that has a limit, day → week → month.
pub fn statuses(rows: &[Row], limits: &Limits, now: DateTime<Local>) -> Vec<LimitStatus> {
    Period::ALL
        .iter()
        .filter_map(|&period| {
            let limit_usd = limits.get(period)?;
            let spent_usd = spent(rows, period, now);
            Some(LimitStatus {
                period,
                limit_usd,
                spent_usd,
                level: level(spent_usd, limit_usd, limits.warn_fraction()),
            })
        })
        .collect()
}

/// Every row since the earliest period start that has a limit (nothing when no
/// limit is set). Independent of the report's own window and tool filter.
pub fn collect_rows(limits: &Limits, now: DateTime<Local>) -> Vec<Row> {
    let Some(start) = Period::ALL
        .iter()
        .filter(|p| limits.get(**p).is_some())
        .map(|p| p.start(now))
        .min()
    else {
        return Vec::new();
    };
    crate::collect(ToolFilter::All, start).rows
}

/// Which alert levels were already raised, per period instance. Persisted so the
/// CLI watcher, TUI, GUI and web server never alert twice for the same thing.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlertLog {
    /// `"daily@<period start>"` → highest level already alerted.
    raised: BTreeMap<String, Level>,
}

fn key(period: Period, now: DateTime<Local>) -> String {
    format!("{}@{}", period.label(), period.start(now).to_rfc3339())
}

impl AlertLog {
    pub fn load_from(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save_to(&self, path: &Path) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    /// Forget periods that are over.
    fn prune(&mut self, now: DateTime<Local>) {
        let live: Vec<String> = Period::ALL.iter().map(|p| key(*p, now)).collect();
        self.raised.retain(|k, _| live.contains(k));
    }
}

/// `<config dir>/alerts.json`.
pub fn alert_log_path() -> Option<PathBuf> {
    crate::settings::config_dir().map(|d| d.join("alerts.json"))
}

/// The alerts `statuses` calls for that `log` has not raised yet; records them.
pub fn pending(statuses: &[LimitStatus], log: &mut AlertLog, now: DateTime<Local>) -> Vec<Alert> {
    log.prune(now);
    let mut out = Vec::new();
    for s in statuses {
        if s.level == Level::Ok {
            continue;
        }
        let seen = log.raised.entry(key(s.period, now)).or_insert(Level::Ok);
        if s.level > *seen {
            *seen = s.level;
            out.push(Alert {
                period: s.period,
                level: s.level,
                spent_usd: s.spent_usd,
                limit_usd: s.limit_usd,
            });
        }
    }
    out
}

/// Statuses for `rows` plus the alerts not yet raised, using the log at `log_path`.
pub fn check_at(
    rows: &[Row],
    limits: &Limits,
    now: DateTime<Local>,
    log_path: Option<&Path>,
) -> (Vec<LimitStatus>, Vec<Alert>) {
    let st = statuses(rows, limits, now);
    let Some(path) = log_path else {
        return (st, Vec::new());
    };
    let mut log = AlertLog::load_from(path);
    let before = log.clone();
    let alerts = pending(&st, &mut log, now);
    if log != before {
        log.save_to(path);
    }
    (st, alerts)
}

/// [`check_at`] with the real clock and the default log.
pub fn check(rows: &[Row], limits: &Limits) -> (Vec<LimitStatus>, Vec<Alert>) {
    check_at(rows, limits, Local::now(), alert_log_path().as_deref())
}

/// Send each alert as a desktop notification (macOS, Linux, Windows). Best effort:
/// a missing notification service is not an error. Returns how many were sent.
pub fn notify(alerts: &[Alert], money: impl Fn(f64) -> String) -> usize {
    alerts
        .iter()
        .filter(|a| {
            notify_rust::Notification::new()
                .appname("TokenBurn")
                .summary(&a.title())
                .body(&a.body(&money))
                .show()
                .is_ok()
        })
        .count()
}

/// Read the saved limits, check them against this period's spend, and — when `live`
/// — log the new alerts and send them as desktop notifications (if the limits ask
/// for them). With `live` false nothing is written or sent. This is the one call
/// the TUI, GUI and web server make after every load.
pub fn run_check(live: bool, money: impl Fn(f64) -> String) -> LimitCheck {
    let limits = crate::settings::load().limits;
    let now = Local::now();
    let rows = collect_rows(&limits, now);
    let (statuses, alerts) = if live {
        check(&rows, &limits)
    } else {
        (statuses(&rows, &limits, now), Vec::new())
    };
    if live && limits.notify {
        notify(&alerts, money);
    }
    LimitCheck {
        limits,
        statuses,
        alerts,
    }
}

/// `today $12.40 / $20.00 (62%) · this week …`, one entry per limit.
pub fn status_line(statuses: &[LimitStatus], money: impl Fn(f64) -> String) -> String {
    statuses
        .iter()
        .map(|s| {
            format!(
                "{} {} / {} ({}%)",
                s.period.span(),
                money(s.spent_usd),
                money(s.limit_usd),
                s.percent()
            )
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(y: i32, m: u32, d: u32, h: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(y, m, d, h, 30, 0).unwrap()
    }

    fn row(ts: DateTime<Local>, cost: f64) -> Row {
        Row {
            tool: crate::Tool::Pi,
            project: "p".into(),
            id: "x".into(),
            ts: ts.with_timezone(&Utc),
            input: 0,
            output: 0,
            cache_read: 0,
            cache_write: 0,
            cost,
        }
    }

    // 2026-05-06 is a Wednesday.
    #[test]
    fn periods_start_at_local_midnight_monday_and_the_first() {
        let now = at(2026, 5, 6, 15);
        let local = |p: Period| p.start(now).with_timezone(&Local);
        assert_eq!(local(Period::Day).date_naive().day(), 6);
        assert_eq!(local(Period::Week).date_naive().day(), 4, "Monday");
        assert_eq!(local(Period::Month).date_naive().day(), 1);
        assert_eq!(local(Period::Day).format("%H:%M").to_string(), "00:00");
        // On a Monday the week starts that day; on a Sunday, six days earlier.
        assert_eq!(
            Period::Week.start(at(2026, 5, 4, 1)),
            Period::Day.start(at(2026, 5, 4, 1))
        );
        assert_eq!(
            Period::Week
                .start(at(2026, 5, 10, 23))
                .with_timezone(&Local)
                .date_naive()
                .day(),
            4
        );
    }

    #[test]
    fn spent_counts_only_the_current_period() {
        let now = at(2026, 5, 6, 15);
        let rows = vec![
            row(at(2026, 5, 6, 9), 1.0),  // today
            row(at(2026, 5, 5, 9), 2.0),  // this week, not today
            row(at(2026, 5, 2, 9), 4.0),  // this month, not this week
            row(at(2026, 4, 30, 9), 8.0), // last month
        ];
        assert_eq!(spent(&rows, Period::Day, now), 1.0);
        assert_eq!(spent(&rows, Period::Week, now), 3.0);
        assert_eq!(spent(&rows, Period::Month, now), 7.0);
    }

    #[test]
    fn levels_ok_warn_over_at_the_threshold() {
        assert_eq!(level(7.9, 10.0, 0.8), Level::Ok);
        assert_eq!(level(8.0, 10.0, 0.8), Level::Warn);
        assert_eq!(level(9.99, 10.0, 0.8), Level::Warn);
        assert_eq!(level(10.0, 10.0, 0.8), Level::Over);
        assert_eq!(level(99.0, 10.0, 0.8), Level::Over);
    }

    #[test]
    fn only_periods_with_a_sane_limit_have_a_status() {
        let now = at(2026, 5, 6, 15);
        let mut l = Limits::default();
        assert!(statuses(&[], &l, now).is_empty() && l.is_empty());
        l.set(Period::Day, Some(10.0));
        l.set(Period::Week, Some(0.0));
        l.set(Period::Month, Some(f64::NAN));
        let st = statuses(&[row(at(2026, 5, 6, 9), 8.5)], &l, now);
        assert_eq!(st.len(), 1);
        assert_eq!(
            (st[0].period, st[0].level, st[0].percent()),
            (Period::Day, Level::Warn, 85)
        );
        l.set(Period::Day, None);
        assert!(l.is_empty());
    }

    #[test]
    fn a_bad_warn_fraction_falls_back() {
        for bad in [0.0, 1.0, 2.0, -1.0, f64::NAN] {
            assert_eq!(
                Limits {
                    warn_at: bad,
                    ..Limits::default()
                }
                .warn_fraction(),
                0.8
            );
        }
        assert_eq!(
            Limits {
                warn_at: 0.5,
                ..Limits::default()
            }
            .warn_fraction(),
            0.5
        );
    }

    #[test]
    fn each_level_alerts_once_per_period_then_again_in_the_next() {
        let now = at(2026, 5, 6, 15);
        let mut l = Limits::default();
        l.set(Period::Day, Some(10.0));
        let mut log = AlertLog::default();
        let run = |cost: f64, now: DateTime<Local>, log: &mut AlertLog| {
            let st = statuses(&[row(now, cost)], &l, now);
            pending(&st, log, now)
        };
        assert!(run(5.0, now, &mut log).is_empty(), "under the threshold");
        let warn = run(8.5, now, &mut log);
        assert_eq!(warn.len(), 1);
        assert_eq!(warn[0].level, Level::Warn);
        assert!(run(9.0, now, &mut log).is_empty(), "same level twice");
        let over = run(11.0, now, &mut log);
        assert_eq!((over.len(), over[0].level), (1, Level::Over));
        assert!(run(50.0, now, &mut log).is_empty());
        // Next day: fresh budget, alerts again; yesterday's entry is pruned.
        let tomorrow = at(2026, 5, 7, 10);
        assert_eq!(run(11.0, tomorrow, &mut log).len(), 1);
        assert_eq!(log.raised.len(), 1);
    }

    #[test]
    fn jumping_straight_to_over_alerts_once() {
        let now = at(2026, 5, 6, 15);
        let mut l = Limits::default();
        l.set(Period::Day, Some(10.0));
        let mut log = AlertLog::default();
        let st = statuses(&[row(now, 30.0)], &l, now);
        let a = pending(&st, &mut log, now);
        assert_eq!((a.len(), a[0].level), (1, Level::Over));
    }

    #[test]
    fn the_log_is_shared_through_its_file() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("alerts.json");
        let now = at(2026, 5, 6, 15);
        let mut l = Limits::default();
        l.set(Period::Day, Some(10.0));
        l.set(Period::Month, Some(100.0));
        let rows = [row(now, 12.0)];
        let (st, first) = check_at(&rows, &l, now, Some(&path));
        assert_eq!((st.len(), first.len()), (2, 1), "day over; month fine");
        // A second process (another front-end) raises nothing.
        assert!(check_at(&rows, &l, now, Some(&path)).1.is_empty());
        // No log path: statuses only.
        assert!(check_at(&rows, &l, now, None).1.is_empty());
        // Garbage in the file is treated as empty.
        std::fs::write(&path, "nope").unwrap();
        assert_eq!(check_at(&rows, &l, now, Some(&path)).1.len(), 1);
    }

    #[test]
    fn alert_text_and_status_line() {
        let a = Alert {
            period: Period::Week,
            level: Level::Over,
            spent_usd: 55.0,
            limit_usd: 50.0,
        };
        assert_eq!(a.title(), "TokenBurn: weekly limit exceeded");
        assert_eq!(
            a.body(|v| format!("${v:.2}")),
            "Spent $55.00 of $50.00 this week (110%)"
        );
        let warn = Alert {
            level: Level::Warn,
            ..a
        };
        assert!(warn.title().contains("nearly reached"));
        let st = LimitStatus {
            period: Period::Day,
            limit_usd: 20.0,
            spent_usd: 12.4,
            level: Level::Ok,
        };
        assert_eq!(
            status_line(&[st], |v| format!("${v:.2}")),
            "today $12.40 / $20.00 (62%)"
        );
    }

    #[test]
    fn collecting_with_no_limits_reads_nothing() {
        assert!(collect_rows(&Limits::default(), Local::now()).is_empty());
    }

    #[test]
    fn limits_round_trip_through_settings_json() {
        let mut s = crate::Settings::default();
        s.limits.set(Period::Day, Some(20.0));
        let json = serde_json::to_string(&s).unwrap();
        let back: crate::Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.limits.get(Period::Day), Some(20.0));
        // Old settings files without limits still load.
        let old: crate::Settings = serde_json::from_str(r#"{"theme":"Default"}"#).unwrap();
        assert!(old.limits.is_empty() && old.limits.notify);
    }
}
