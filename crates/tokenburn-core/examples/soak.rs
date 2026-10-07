//! Memory soak test: does repeating what a front-end does every few seconds make the
//! process grow? Each round re-reads the logs, rebuilds the report, the charts and
//! the drill-down (with and without a selection) — exactly the refresh path of the
//! TUI, GUI and web — and prints the resident memory every few rounds.
//!
//!     cargo run --release -p tokenburn-core --example soak -- [rounds] [month|all]
//!
//! A leak shows as a steady climb; a healthy run flattens out after the first rounds.

use std::process::Command;

use chrono::Local;
use tokenburn_core::{
    build_report, by_project, collect, drill, pie, time_series, Query, Selection, ToolFilter,
    Window,
};

fn rss_mb() -> f64 {
    let out = Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .expect("ps");
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<f64>()
        .unwrap_or(0.0)
        / 1024.0
}

fn main() {
    let mut args = std::env::args().skip(1);
    let rounds: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(100);
    let window = match args.next().as_deref() {
        Some("all") => Window::All,
        _ => Window::Month,
    };
    let query = Query {
        bucket: tokenburn_core::Bucket::Day,
        window,
        ..Query::default()
    };

    println!("round   rows   rss MB");
    let mut first = 0.0;
    for i in 0..rounds {
        let collected = collect(ToolFilter::All, window.start(Local::now()));
        let rows = collected.rows;
        let _ = build_report(&rows, &query);
        let _ = time_series(&rows, &query);
        let _ = pie(&by_project(&rows), 8);
        let all = drill(&rows, &query, &Selection::default());
        // Click a bar and a slice, as a user would.
        let mut sel = Selection::default();
        if let Some(l) = all.series.labels.get(3) {
            sel.toggle_bucket(l);
        }
        if let Some(k) = all.pie.slices.first().and_then(|s| s.key.clone()) {
            sel.toggle_project(&k);
        }
        let _ = drill(&rows, &query, &sel);
        if i == 0 || i % (rounds / 10).max(1) == 0 || i == rounds - 1 {
            let rss = rss_mb();
            if i == 0 {
                first = rss;
            }
            println!("{i:>5}  {:>6}  {rss:>7.1}", rows.len());
        }
    }
    println!("first round {first:.1} MB → last round {:.1} MB", rss_mb());
}
