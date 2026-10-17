//! Plain-text table renderer.

use tokenburn_core::utils::fmt::{percent, thousands};
use tokenburn_core::{BucketSavings, Currency, ReportRow, Savings};

const HEAD_BUCKET: [&str; 1] = ["bucket"];
const HEAD: [&str; 7] = [
    "tool",
    "turns",
    "input",
    "output",
    "cache_read",
    "cache_write",
    "total_tokens",
];

/// Render report rows as an aligned table, costs in `currency` (the last column is
/// `cost_usd`, `cost_eur`, …). Numeric columns are right-aligned.
pub fn table(rows: &[ReportRow], currency: &Currency) -> String {
    let bucketed = rows.iter().any(|r| r.bucket.is_some());

    let cost_head = format!("cost_{}", currency.code.to_ascii_lowercase());
    let mut head: Vec<&str> = Vec::new();
    if bucketed {
        head.extend(HEAD_BUCKET);
    }
    head.extend(HEAD);
    head.push(&cost_head);

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
                currency.format(s.cost_usd),
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

/// The "saved" block: one line per source, then the total. `spent_usd` is the
/// report's cost, which gives the share saved. Empty text when nothing was saved.
pub fn savings(sv: &Savings, currency: &Currency, spent_usd: f64) -> String {
    if sv.is_empty() {
        return "saved: nothing (no cache hits, rtk or pi compaction found)\n".to_string();
    }
    let mut out = String::from("saved (estimate)\n");
    for s in &sv.sources {
        out.push_str(&format!(
            "  {:<14}{:>16} tokens  {:>12}  ({} {})\n",
            s.source.label(),
            thousands(s.tokens),
            currency.format(s.usd),
            thousands(s.events),
            s.source.event_noun(),
        ));
    }
    out.push_str(&format!(
        "  {:<14}{:>16} tokens  {:>12}",
        "total",
        thousands(sv.tokens),
        currency.format(sv.usd)
    ));
    if let Some(p) = sv.percent_of_would_be(spent_usd) {
        out.push_str(&format!("  ({} of what it would have cost)", percent(p)));
    }
    out.push('\n');
    out
}

/// Savings per interval (minute, hour, day or month), oldest first: one line per
/// bucket with the money and where it came from.
pub fn savings_by_bucket(buckets: &[BucketSavings], currency: &Currency) -> String {
    let mut out = String::from("saved per interval\n");
    for b in buckets {
        let split = b
            .savings
            .sources
            .iter()
            .map(|s| format!("{} {}", s.source.label(), currency.format(s.usd)))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "  {:<18}{:>16} tokens  {:>12}  ({})\n",
            b.label,
            thousands(b.savings.tokens),
            currency.format(b.savings.usd),
            split,
        ));
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
        let t = table(
            &[rr(None, "pi", 1234), rr(None, "TOTAL", 1234)],
            &Currency::USD,
        );
        assert!(t.starts_with("tool"));
        assert!(t.contains("1,234"));
        assert!(t.contains("$1.50"));
        assert!(t.contains("cost_usd"));
        assert!(!t.contains("bucket"));
    }

    #[test]
    fn bucketed_table_has_bucket_column() {
        let t = table(&[rr(Some("2026-05"), "pi", 10)], &Currency::USD);
        assert!(t.starts_with("bucket"));
        assert!(t.contains("2026-05"));
    }

    #[test]
    fn costs_are_shown_in_the_chosen_currency() {
        let eur = tokenburn_core::currency_by_code("EUR").unwrap();
        let t = table(&[rr(None, "pi", 100)], &eur);
        assert!(t.contains("cost_eur"), "{t}");
        assert!(t.contains("€1.38"), "1.50 USD at 0.92: {t}");
        assert!(!t.contains("cost_usd"));
        let jpy = tokenburn_core::currency_by_code("JPY").unwrap();
        assert!(table(&[rr(None, "pi", 100)], &jpy).contains("¥225"));
    }

    #[test]
    fn savings_block_lists_sources_and_the_share() {
        use tokenburn_core::{SavingsSource, SourceSavings};
        let sv = Savings::from_sources([
            SourceSavings {
                source: SavingsSource::PromptCache,
                tokens: 1_000_000,
                usd: 2.5,
                events: 10,
            },
            SourceSavings {
                source: SavingsSource::Rtk,
                tokens: 500,
                usd: 2.5,
                events: 3,
            },
        ]);
        let t = savings(&sv, &Currency::USD, 5.0);
        assert!(t.starts_with("saved (estimate)"), "{t}");
        assert!(t.contains("Prompt cache") && t.contains("1,000,000 tokens"));
        assert!(t.contains("(3 commands)") && t.contains("$2.50"));
        assert!(
            t.contains("$5.00") && t.contains("50% of what it would have cost"),
            "{t}"
        );
        let eur = tokenburn_core::currency_by_code("EUR").unwrap();
        assert!(savings(&sv, &eur, 5.0).contains("€2.30"));
    }

    #[test]
    fn savings_block_when_nothing_was_saved() {
        let t = savings(&Savings::default(), &Currency::USD, 1.0);
        assert!(t.starts_with("saved: nothing"));
    }
}
