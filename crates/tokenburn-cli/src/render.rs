//! Plain-text table renderer.

use tokenburn_core::utils::fmt::{thousands, usd};
use tokenburn_core::ReportRow;

const HEAD_BUCKET: [&str; 1] = ["bucket"];
const HEAD: [&str; 8] = [
    "tool",
    "turns",
    "input",
    "output",
    "cache_read",
    "cache_write",
    "total_tokens",
    "cost_usd",
];

/// Render report rows as an aligned table. Numeric columns are right-aligned.
pub fn table(rows: &[ReportRow]) -> String {
    let bucketed = rows.iter().any(|r| r.bucket.is_some());

    let mut head: Vec<&str> = Vec::new();
    if bucketed {
        head.extend(HEAD_BUCKET);
    }
    head.extend(HEAD);

    let body: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            let s = &r.summary;
            let mut cells = Vec::new();
            if bucketed {
                cells.push(r.bucket.clone().unwrap_or_default());
            }
            cells.extend([
                r.tool.clone(),
                thousands(s.turns),
                thousands(s.input),
                thousands(s.output),
                thousands(s.cache_read),
                thousands(s.cache_write),
                thousands(s.total_tokens),
                usd(s.cost_usd),
            ]);
            cells
        })
        .collect();

    // Text columns (bucket, tool) are left-aligned, the rest right-aligned.
    let text_cols = if bucketed { 2 } else { 1 };
    let widths: Vec<usize> = (0..head.len())
        .map(|i| {
            body.iter()
                .map(|r| r[i].chars().count())
                .chain([head[i].len()])
                .max()
                .unwrap_or(0)
        })
        .collect();

    let line = |cells: &[String]| -> String {
        let mut parts = Vec::new();
        for (i, c) in cells.iter().enumerate() {
            if i < text_cols {
                parts.push(format!("{c:<w$}", w = widths[i]));
            } else {
                parts.push(format!("{c:>w$}", w = widths[i]));
            }
        }
        parts.join("  ").trim_end().to_string()
    };

    let mut out = String::new();
    let head_cells: Vec<String> = head.iter().map(|s| s.to_string()).collect();
    out.push_str(&line(&head_cells));
    out.push('\n');
    out.push_str(&"─".repeat(widths.iter().sum::<usize>() + 2 * (widths.len() - 1)));
    out.push('\n');
    for r in &body {
        out.push_str(&line(r));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokenburn_core::Summary;

    fn rr(bucket: Option<&str>, tool: &str, input: u64) -> ReportRow {
        ReportRow {
            bucket: bucket.map(String::from),
            tool: tool.into(),
            summary: Summary {
                turns: 2,
                input,
                output: 5,
                cache_read: 0,
                cache_write: 0,
                total_tokens: input + 5,
                cost_usd: 1.5,
            },
        }
    }

    #[test]
    fn unbucketed_table() {
        let t = table(&[rr(None, "pi", 1234), rr(None, "TOTAL", 1234)]);
        assert!(t.starts_with("tool"));
        assert!(t.contains("1,234"));
        assert!(t.contains("$1.5"));
        assert!(!t.contains("bucket"));
    }

    #[test]
    fn bucketed_table_has_bucket_column() {
        let t = table(&[rr(Some("2026-05"), "pi", 10)]);
        assert!(t.starts_with("bucket"));
        assert!(t.contains("2026-05"));
    }
}
