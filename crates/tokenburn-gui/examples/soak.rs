//! Memory soak test for the GUI's state and view code (everything except the GPU):
//! every round loads the logs, applies them (report, charts, drill-down), clicks a bar
//! and a slice, and builds the whole widget tree at every layout class. A leak shows as
//! a steady climb in the resident memory.
//!
//!     cargo run --release -p tokenburn-gui --example soak -- [rounds]

use std::process::Command;

use chrono::Local;
use tokenburn_core::{collect, Bucket, LayoutClass, Window};
use tokenburn_gui::{Message, TokenBurn};

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
    let rounds: usize = std::env::args()
        .nth(1)
        .and_then(|n| n.parse().ok())
        .unwrap_or(60);
    let mut app = TokenBurn::new();
    app.query.bucket = Bucket::Day;
    app.query.window = Window::All;

    println!("round   rows   rss MB");
    for i in 0..rounds {
        let collected = collect(app.query.tool, app.query.window.start(Local::now()));
        let query = app.query;
        let _ = app.update(Message::Loaded(query, Ok(collected)));

        // A user clicking around between refreshes.
        if let Some(label) = app
            .series
            .labels
            .get(i % app.series.labels.len().max(1))
            .cloned()
        {
            let _ = app.update(Message::BucketClicked(label));
        }
        if let Some(key) = app.pie.slices.first().and_then(|s| s.key.clone()) {
            let _ = app.update(Message::SliceClicked(key));
        }
        let _ = app.update(Message::MetricSelected(if i % 2 == 0 {
            tokenburn_core::Metric::Cost
        } else {
            tokenburn_core::Metric::Tokens
        }));
        let _ = app.update(Message::CurrencySelected(
            tokenburn_gui::features::currency::CurrencyChoice::at(i % 5),
        ));
        for layout in [LayoutClass::Wide, LayoutClass::Medium, LayoutClass::Narrow] {
            let _ = app.view_for(layout);
        }
        let _ = app.update(Message::ClearSelection);

        if i == 0 || i % (rounds / 10).max(1) == 0 || i == rounds - 1 {
            println!("{i:>5}  {:>6}  {:>7.1}", app.rows.len(), rss_mb());
        }
    }
}
