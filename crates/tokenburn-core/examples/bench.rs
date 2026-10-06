//! Refresh benchmark: how long does re-reading your logs take, cold and warm?
//!
//!     cargo run --release -p tokenburn-core --example bench            # this month
//!     cargo run --release -p tokenburn-core --example bench -- all     # all history
//!     cargo run --release -p tokenburn-core --example bench -- all 10   # …10 refreshes
//!
//! The first run is "cold" (every file parsed); later runs show what an
//! auto-refresh costs when little has changed.

use std::time::Instant;

use chrono::Local;
use tokenburn_core::{build_report, collect, Query, ToolFilter, Window};

fn main() {
    let mut args = std::env::args().skip(1);
    let window = match args.next().as_deref() {
        Some("all") => Window::All,
        Some("today") => Window::Today,
        _ => Window::Month,
    };
    let runs: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(5);
    let start = window.start(Local::now());

    for i in 0..runs {
        let t = Instant::now();
        let collected = collect(ToolFilter::All, start);
        let collect_t = t.elapsed();
        let t = Instant::now();
        let report = build_report(&collected.rows, &Query::default());
        println!(
            "run {i}: collect {:>8.1?} · report {:>8.1?} · {} rows · {} report lines",
            collect_t,
            t.elapsed(),
            collected.rows.len(),
            report.len()
        );
    }
}
