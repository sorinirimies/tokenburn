//! `--limits`, `--limit-daily/weekly/monthly` and `--watch`.

use std::io::Write;

use anyhow::Result;
use chrono::Local;
use tokenburn_core::{
    limit_rows, limit_statuses, settings, Alert, Currency, Level, LimitStatus, Period,
};

use crate::args::Args;

/// `true` when any limit-related flag is present (the report is then skipped).
pub fn wanted(args: &Args) -> bool {
    args.limits
        || args.watch.is_some()
        || args.limit_daily.is_some()
        || args.limit_weekly.is_some()
        || args.limit_monthly.is_some()
}

/// Save new limits, print the state, or watch, depending on the flags.
pub fn run(args: &Args, currency: &Currency, out: &mut impl Write) -> Result<()> {
    let mut s = settings::load();
    let changed = [
        (Period::Day, args.limit_daily),
        (Period::Week, args.limit_weekly),
        (Period::Month, args.limit_monthly),
    ];
    let mut any = false;
    for (period, usd) in changed {
        if let Some(usd) = usd {
            s.limits.set(period, Some(usd));
            any = true;
        }
    }
    if any {
        settings::save_limits(&s.limits)?;
        writeln!(out, "Saved limits (US dollars).")?;
    }
    if let Some(secs) = args.watch {
        return watch(secs, currency, out);
    }
    write!(
        out,
        "{}",
        render(&statuses_now(), currency, s.limits.is_empty())
    )?;
    Ok(())
}

fn statuses_now() -> Vec<LimitStatus> {
    let limits = settings::load().limits;
    let now = Local::now();
    limit_statuses(&limit_rows(&limits, now), &limits, now)
}

/// A table of every limit with its progress; the hint when none is set.
pub fn render(statuses: &[LimitStatus], currency: &Currency, no_limits: bool) -> String {
    if no_limits {
        return "No limits set. Try: tokenburn --limit-daily 20 --limit-weekly 100 --limit-monthly 300\n".into();
    }
    let mut out = String::new();
    for s in statuses {
        let mark = match s.level {
            Level::Ok => "ok",
            Level::Warn => "WARNING",
            Level::Over => "OVER",
        };
        out.push_str(&format!(
            "{:<8} {:>10} of {:<10} {:>4}%  {}\n",
            s.period.label(),
            currency.format(s.spent_usd),
            currency.format(s.limit_usd),
            s.percent(),
            mark
        ));
    }
    out
}

/// One alert as a line of text.
pub fn alert_line(a: &Alert, currency: &Currency) -> String {
    format!("{}: {}", a.title(), a.body(|v| currency.format(v)))
}

/// One pass of the watcher: raise alerts not raised before, print them and send
/// desktop notifications when enabled. Returns the alerts.
pub fn watch_once(currency: &Currency, notify: bool, out: &mut impl Write) -> Result<Vec<Alert>> {
    let limits = settings::load().limits;
    let rows = limit_rows(&limits, Local::now());
    let (_, alerts) = tokenburn_core::check_limits(&rows, &limits);
    for a in &alerts {
        writeln!(out, "{}", alert_line(a, currency))?;
    }
    if notify && limits.notify {
        tokenburn_core::notify_alerts(&alerts, |v| currency.format(v));
    }
    out.flush()?;
    Ok(alerts)
}

fn watch(secs: u64, currency: &Currency, out: &mut impl Write) -> Result<()> {
    writeln!(out, "Watching limits every {secs}s (Ctrl-C to stop).")?;
    loop {
        watch_once(currency, true, out)?;
        std::thread::sleep(std::time::Duration::from_secs(secs));
    }
}
