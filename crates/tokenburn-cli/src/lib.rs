//! TokenBurn CLI — a [`clap`] front-end over [`tokenburn_core`].
//!
//! ```text
//! tokenburn                      current month total, pi+zed combined
//! tokenburn --tool pi            pi only
//! tokenburn --hourly             today, bucketed by hour
//! tokenburn --daily              this month, bucketed by day
//! tokenburn --monthly            all history, bucketed by month
//! tokenburn --daily --all        all history, bucketed by day
//! tokenburn --json               machine-readable output
//! ```

pub mod args;
pub mod render;

use std::io::Write;

use anyhow::Result;
use chrono::Local;
use tokenburn_core::{build_report, collect};

pub use args::Args;

/// Run the CLI, writing the report to `out` (warnings go to stderr).
pub fn run(args: &Args, out: &mut impl Write) -> Result<()> {
    let query = args.query();
    let now = Local::now();
    let collected = collect(query.tool, query.window.start(now));

    for w in &collected.warnings {
        eprintln!("tokenburn: {w}");
    }

    let report = build_report(&collected.rows, &query);
    if report.is_empty() {
        if args.json {
            writeln!(out, "[]")?;
        } else {
            writeln!(
                out,
                "No tokenburn results found for tool={}, window={}.",
                query.tool, query.window
            )?;
        }
        return Ok(());
    }

    if args.json {
        writeln!(out, "{}", serde_json::to_string_pretty(&report)?)?;
    } else {
        write!(out, "{}", render::table(&report))?;
    }
    Ok(())
}
