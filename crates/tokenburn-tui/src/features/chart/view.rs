//! Bar chart of total tokens per bucket (or per tool when un-bucketed).

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Bar, BarChart, BarGroup, Block, Borders};
use ratatui::Frame;
use tokenburn_core::utils::fmt::compact;
use tokenburn_core::Metric;

use crate::app::App;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let th = app.theme;
    // Un-bucketed: chart the per-tool lines (skip TOTAL, it would dwarf them).
    let lines: Vec<_> = app.report.iter().filter(|r| r.tool != "TOTAL").collect();

    // Show only the most recent buckets that fit.
    let bar_w: u16 = 9;
    let fit = (area.width.saturating_sub(2) / (bar_w + 1)).max(1) as usize;
    let skip = lines.len().saturating_sub(fit);

    let metric = app.metric;
    let money = |usd: f64| app.currency.format(usd);
    let bars: Vec<Bar> = match metric {
        // What was saved per chart interval (cache, rtk and compaction together).
        Metric::Saved => {
            let all = app.savings_by_bucket();
            let skip = all.len().saturating_sub(fit);
            all.iter()
                .skip(skip)
                .map(|b| {
                    Bar::default()
                        .value((b.savings.usd * 100.0).round() as u64)
                        .text_value(money(b.savings.usd))
                        .label(short(&b.label))
                        .style(Style::default().fg(th.success))
                })
                .collect()
        }
        _ => lines
            .iter()
            .skip(skip)
            .map(|r| {
                let label = match &r.bucket {
                    Some(b) => format!("{} {}", short(b), r.tool),
                    None => r.tool.clone(),
                };
                let (value, text) = if metric == Metric::Cost {
                    (
                        (r.summary.cost_usd * 100.0).round() as u64,
                        money(r.summary.cost_usd),
                    )
                } else {
                    (r.summary.total_tokens, compact(r.summary.total_tokens))
                };
                Bar::default()
                    .value(value)
                    .text_value(text)
                    .label(label)
                    .style(Style::default().fg(th.tool(&r.tool)))
            })
            .collect(),
    };

    let chart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(th.border))
                .title(title(metric)),
        )
        .bar_width(bar_w)
        .bar_gap(1)
        .label_style(Style::default().fg(th.muted))
        .value_style(Style::default().fg(th.bg).bg(th.accent))
        .data(BarGroup::default().bars(&bars));
    frame.render_widget(chart, area);
}

/// Trim a bucket label to its trailing, most-significant-for-recency part.
fn short(bucket: &str) -> String {
    // "2026-05-01 13:00" → "05-01 13h"; "2026-05-01" → "05-01"; "2026-05" → "2026-05"
    match bucket.len() {
        // An hour bucket ends in ":00"; a minute bucket shows just its time.
        16 if bucket.ends_with(":00") => format!("{}h", &bucket[5..13]),
        16 => bucket[11..].to_string(),
        10 => bucket[5..].to_string(),
        _ => bucket.to_string(),
    }
}

/// The panel title for `metric`.
fn title(metric: Metric) -> &'static str {
    match metric {
        Metric::Tokens => " Total tokens ",
        Metric::Cost => " Cost ",
        Metric::Saved => " Saved (cache + rtk + compaction) ",
    }
}

#[cfg(test)]
mod tests {
    use super::short;

    #[test]
    fn shortens_labels() {
        assert_eq!(short("2026-05-01 13:00"), "05-01 13h");
        assert_eq!(short("2026-05-01"), "05-01");
        assert_eq!(short("2026-05"), "2026-05");
    }
}
