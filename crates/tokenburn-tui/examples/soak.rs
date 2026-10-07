//! Memory soak test for the terminal UI: reload the logs and redraw the screen over and
//! over (what auto-refresh does every few seconds), filtering and switching views along
//! the way, and print the resident memory. A leak shows as a steady climb.
//!
//!     cargo run --release -p tokenburn-tui --example soak -- [rounds]

use std::process::Command;
use std::time::{Duration, Instant};

use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tokenburn_core::{Bucket, Window};
use tokenburn_tui::app::App;
use tokenburn_tui::layout;

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
    let mut term = Terminal::new(TestBackend::new(140, 40)).unwrap();
    let mut app = App::new();
    app.query.bucket = Bucket::Day;
    app.query.window = Window::All;

    println!("round   rows   rss MB");
    for i in 0..rounds {
        app.reload();
        let started = Instant::now();
        while app.is_loading && started.elapsed() < Duration::from_secs(60) {
            app.poll_background();
            std::thread::sleep(Duration::from_millis(5));
        }
        // What a user does between refreshes: filter, look at projects, redraw.
        app.filter.text = if i % 2 == 0 {
            "pi".into()
        } else {
            String::new()
        };
        app.rebuild();
        if i % 3 == 0 {
            app.toggle_view();
        }
        for _ in 0..3 {
            term.draw(|f| layout::render(&mut app, f)).unwrap();
        }
        if i == 0 || i % (rounds / 10).max(1) == 0 || i == rounds - 1 {
            println!("{i:>5}  {:>6}  {:>7.1}", app.all_rows.len(), rss_mb());
        }
    }
}
