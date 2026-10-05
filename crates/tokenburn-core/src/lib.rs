//! TokenBurn core — UI-agnostic logic shared by the CLI, TUI and GUI front-ends.
//!
//! Reads **local logs only** (no network):
//!
//! * [pi](https://github.com/earendil-works/pi-coding-agent) session files,
//!   `~/.pi/agent/sessions/**/*.jsonl` — one row per assistant turn.
//! * Zed's agent thread database (`threads.db`, sqlite + zstd JSON blobs) —
//!   one row per thread (input/output tokens only, no `$` cost).
//!
//! # Typical flow
//!
//! ```no_run
//! use tokenburn_core::{collect, build_report, Query};
//!
//! let query = Query::default();
//! let now = chrono::Local::now();
//! let collected = collect(query.tool, query.window.start(now));
//! let report = build_report(&collected.rows, &query);
//! println!("{} rows", report.len());
//! ```

pub mod features;
pub mod utils;

pub use features::report::{
    build_report, by_project, Bucket, Query, ReportRow, ToolFilter, Window,
};
pub use features::usage::{Row, Summary, Tool};

use chrono::{DateTime, Utc};

/// Result of reading every enabled source.
#[derive(Debug, Default, Clone)]
pub struct Collected {
    /// Every usage row at or after the requested start.
    pub rows: Vec<Row>,
    /// Non-fatal problems (missing db, unreadable file…), safe to show to the user.
    pub warnings: Vec<String>,
}

/// Collect rows for `tool` from the default on-disk locations, keeping only
/// rows whose timestamp is `>= start`.
pub fn collect(tool: ToolFilter, start: DateTime<Utc>) -> Collected {
    let mut out = Collected::default();

    if tool.includes(Tool::Pi) {
        match features::pi::collect_pi(start) {
            Ok(rows) => out.rows.extend(rows),
            Err(e) => out.warnings.push(format!("pi: {e:#}")),
        }
    }
    if tool.includes(Tool::Zed) {
        match features::zed::collect_zed(start) {
            Ok(rows) => out.rows.extend(rows),
            Err(e) => out.warnings.push(format!("zed: {e:#}")),
        }
    }

    out.rows.retain(|r| r.ts >= start);
    out.rows.sort_by_key(|r| r.ts);
    out
}
