//! TokenBurn core — UI-agnostic logic shared by the CLI, TUI and GUI front-ends.
//!
//! Reads **local logs only** (no network):
//!
//! * [pi](https://github.com/earendil-works/pi-coding-agent) session files,
//!   `~/.pi/agent/sessions/**/*.jsonl` — one row per assistant turn.
//! * Zed's agent thread database (`threads.db`, sqlite + zstd JSON blobs) —
//!   one row per thread (input/output tokens only, no `$` cost).
//! * Claude Code, Codex CLI, Copilot CLI, Gemini CLI, OpenCode and Amp — see
//!   [`Tool`] and `features/<tool>/`.
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

pub mod appinfo;
pub mod features;
pub mod utils;

pub use features::currency::{find as currency_by_code, Currency, CURRENCIES};
pub use features::report::{
    axis_label, axis_tokens, bucket_at, bucket_label_at, bucket_label_of, build_report, by_project,
    cards, chart_bucket, drill, drill_priced, nice_max, pie, project_key, projects_add_detail,
    short_label, short_name, slice_angles, time_series, time_series_priced, Bucket, Card, CardKind,
    Drill, Metric, Pie, Query, ReportRow, RowFilter, SavedInput, Selection, Slice, TimeSeries,
    ToolFilter, Window,
};
pub use features::savings::{
    avoided_usd, cache_savings, cache_usd, collect_external, savings_by_bucket, BucketSavings,
    ExternalSavings, Savings, SavingsSource, SourceSavings, CACHE_READ_PRICE_RATIO,
    DEFAULT_INPUT_PRICE_PER_MTOK,
};
pub use features::settings::{self, Settings};
pub use features::theme::{
    current_system_theme_index, detect_system_dark, find_theme, is_system_name, system_theme_index,
    theme_by_index, theme_index_by_name, AppTheme, Rgb, ThemeChoice, SYSTEM_THEME_NAME,
    THEME_COUNT, THEME_NAMES,
};
pub use features::usage::{Row, Summary, Tool};
pub use utils::layout::LayoutClass;

use chrono::{DateTime, Utc};

/// Result of reading every enabled source.
#[derive(Debug, Default, Clone)]
pub struct Collected {
    /// Every usage row at or after the requested start.
    pub rows: Vec<Row>,
    /// Non-fatal problems (missing db, unreadable file…), safe to show to the user.
    pub warnings: Vec<String>,
    /// Savings from sources outside the usage rows (rtk, pi compaction).
    pub external: ExternalSavings,
}

/// Collect rows for `tool` from the default on-disk locations, keeping only
/// rows whose timestamp is `>= start`.
///
/// Every tool in the filter is read on its own thread (a Zed database or a
/// year of Claude transcripts can take a moment). A tool that is not installed
/// simply contributes nothing; a tool whose logs cannot be read adds a warning.
pub fn collect(tool: ToolFilter, start: DateTime<Utc>) -> Collected {
    use features::{amp, claude, cline, codex, copilot, gemini, opencode, pi, qwen, zed};

    type Job = fn(DateTime<Utc>) -> anyhow::Result<Vec<Row>>;
    let jobs: [(Tool, Job); Tool::COUNT] = [
        (Tool::Pi, pi::collect_pi),
        (Tool::Zed, zed::collect_zed),
        (Tool::Claude, claude::collect_claude),
        (Tool::Codex, codex::collect_codex),
        (Tool::Copilot, copilot::collect_copilot),
        (Tool::Gemini, gemini::collect_gemini),
        (Tool::OpenCode, opencode::collect_opencode),
        (Tool::Amp, amp::collect_amp),
        (Tool::Cline, cline::collect_cline),
        (Tool::Roo, cline::collect_roo),
        (Tool::Kilo, cline::collect_kilo),
        (Tool::Qwen, qwen::collect_qwen),
    ];

    let results: Vec<(Tool, anyhow::Result<Vec<Row>>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = jobs
            .iter()
            .filter(|(t, _)| tool.includes(*t))
            .map(|(t, job)| (*t, scope.spawn(move || job(start))))
            .collect();
        handles
            .into_iter()
            .map(|(t, h)| {
                let r = h
                    .join()
                    .unwrap_or_else(|_| Err(anyhow::anyhow!("collector panicked")));
                (t, r)
            })
            .collect()
    });

    let mut out = Collected::default();
    for (t, r) in results {
        match r {
            Ok(rows) => out.rows.extend(rows),
            Err(e) => out.warnings.push(format!("{}: {e:#}", t.as_str())),
        }
    }
    out.rows.retain(|r| r.ts >= start);
    out.rows.sort_by_key(|r| r.ts);
    out.external = collect_external(tool, start);
    out
}
