//! TokenBurn CLI — a [`clap`] front-end over [`tokenburn_core`].
//!
//! ```text
//! tokenburn                      current month total, all tools combined
//! tokenburn --tool pi            pi only
//! tokenburn --minutely          today, bucketed by minute
//! tokenburn --hourly             today, bucketed by hour
//! tokenburn --daily              this month, bucketed by day
//! tokenburn --monthly            all history, bucketed by month
//! tokenburn --daily --all        all history, bucketed by day
//! tokenburn --json               machine-readable output
//! tokenburn --savings            also show tokens/money saved (cache, rtk, pi compaction)
//! ```

pub mod args;
pub mod render;

use std::io::Write;

use anyhow::Result;
use chrono::Local;
use tokenburn_core::{build_report, collect, savings_by_bucket, Savings, CURRENCIES};

pub use args::Args;

/// Run the CLI, writing the report to `out` (warnings go to stderr).
pub fn run(args: &Args, out: &mut impl Write) -> Result<()> {
    if args.list_currencies {
        for c in CURRENCIES {
            writeln!(out, "{:<4}{:<22}{} per USD", c.code, c.name, c.per_usd)?;
        }
        return Ok(());
    }
    let currency = args
        .currency
        .unwrap_or_else(tokenburn_core::settings::currency);
    let query = args.query();
    let now = Local::now();
    let collected = collect(query.tool, query.window.start(now));

    for w in &collected.warnings {
        eprintln!("tokenburn: {w}");
    }

    let report = build_report(&collected.rows, &query);
    let savings = args.savings.then(|| {
        let price = args
            .input_price
            .unwrap_or_else(tokenburn_core::settings::input_price);
        let by_bucket = if query.bucket == tokenburn_core::Bucket::None {
            Vec::new()
        } else {
            savings_by_bucket(&collected.rows, &query, &collected.external, price)
        };
        (
            Savings::build(&collected.rows, &collected.external, price),
            by_bucket,
        )
    });
    // JSON: the savings object, plus `by_bucket` when the report is bucketed.
    let savings_json = savings.as_ref().map(|(sv, by_bucket)| {
        let mut v = serde_json::to_value(sv).unwrap_or_default();
        if !by_bucket.is_empty() {
            v["by_bucket"] = serde_json::to_value(by_bucket).unwrap_or_default();
        }
        v
    });
    if report.is_empty() {
        if args.json {
            match &savings_json {
                Some(sv) => writeln!(
                    out,
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({"report": [], "savings": sv})
                    )?
                )?,
                None => writeln!(out, "[]")?,
            }
        } else {
            writeln!(
                out,
                "No tokenburn results found for tool={}, window={}.",
                query.tool, query.window
            )?;
        }
        return Ok(());
    }

    match (args.json, &savings, &savings_json) {
        (true, None, _) | (true, _, None) => {
            writeln!(out, "{}", serde_json::to_string_pretty(&report)?)?
        }
        (true, Some(_), Some(sv)) => writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"report": report, "savings": sv}))?
        )?,
        (false, sv, _) => {
            write!(out, "{}", render::table(&report, &currency))?;
            if let Some((sv, by_bucket)) = sv {
                let spent = tokenburn_core::Summary::of(&collected.rows).cost_usd;
                write!(out, "\n{}", render::savings(sv, &currency, spent))?;
                if !by_bucket.is_empty() {
                    write!(out, "\n{}", render::savings_by_bucket(by_bucket, &currency))?;
                }
            }
        }
    }
    Ok(())
}
