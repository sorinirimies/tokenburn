//! Bar chart of total tokens per bucket (or per tool when un-bucketed).

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Bar, BarChart, BarGroup, Block, Borders};
use ratatui::Frame;
use tokenburn_core::utils::fmt::compact;

use crate::app::App;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    // Un-bucketed: chart the per-tool lines (skip TOTAL, it would dwarf them).
    let lines: Vec<_> = app.report.iter().filter(|r| r.tool != "TOTAL").collect();

    // Show only the most recent buckets that fit.
    let bar_w: u16 = 9;
    let fit = (area.width.saturating_sub(2) / (bar_w + 1)).max(1) as usize;
    let skip = lines.len().saturating_sub(fit);

    let bars: Vec<Bar> = lines
        .iter()
        .skip(skip)
        .map(|r| {
            let label = match &r.bucket {
                Some(b) => format!("{} {}", short(b), r.tool),
                None => r.tool.clone(),
            };
            Bar::default()
                .value(r.summary.total_tokens)
                .text_value(compact(r.summary.total_tokens))
                .label(label)
                .style(Style::default().fg(if r.tool == "zed" {
                    Color::Magenta
                } else {
                    Color::Cyan
                }))
        })
        .collect();

    let chart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Total tokens "),
        )
        .bar_width(bar_w)
        .bar_gap(1)
        .data(BarGroup::default().bars(&bars));
    frame.render_widget(chart, area);
}

/// Trim a bucket label to its trailing, most-significant-for-recency part.
fn short(bucket: &str) -> String {
    // "2026-05-01 13:00" → "05-01 13h"; "2026-05-01" → "05-01"; "2026-05" → "2026-05"
    match bucket.len() {
        16 => format!("{}h", &bucket[5..13]),
        10 => bucket[5..].to_string(),
        _ => bucket.to_string(),
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
