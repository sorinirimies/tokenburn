//! The savings breakdown popup.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use tokenburn_core::utils::fmt::{compact, percent, thousands};
use tokenburn_core::SavingsSource;

use crate::app::App;

/// The popup's text lines (also what the tests read).
pub fn lines(app: &App) -> Vec<Line<'static>> {
    let t = app.theme;
    let sv = app.savings();
    let muted = Style::default().fg(t.muted);
    let text = Style::default().fg(t.text);
    let good = Style::default().fg(t.success).add_modifier(Modifier::BOLD);
    let mut out = vec![Line::from(Span::styled(
        format!(
            "valued at {}/M input tokens (estimate)",
            app.currency.format(app.input_price)
        ),
        muted,
    ))];
    if app.query.tool != tokenburn_core::ToolFilter::All && sv.of(SavingsSource::Rtk).is_none() {
        out.push(Line::from(Span::styled(
            "rtk is counted only with tool:all (it cannot tell which agent ran a command)",
            muted,
        )));
    }
    out.push(Line::raw(""));
    if sv.is_empty() {
        out.push(Line::from(Span::styled(
            "Nothing saved in this view: no cache hits, rtk history or pi compactions.",
            text,
        )));
        return out;
    }
    for s in &sv.sources {
        out.push(Line::from(vec![
            Span::styled(format!("{:<14}", s.source.label()), text),
            Span::styled(format!("{:>9} tok  ", compact(s.tokens)), text),
            Span::styled(format!("{:>10}  ", app.currency.format(s.usd)), good),
            Span::styled(
                format!("{} {}", thousands(s.events), s.source.event_noun()),
                muted,
            ),
        ]));
    }
    out.push(Line::raw(""));
    out.push(Line::from(vec![
        Span::styled(
            format!("{:<14}", "total"),
            Style::default().fg(t.text).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("{:>9} tok  ", compact(sv.tokens)), text),
        Span::styled(format!("{:>10}", app.currency.format(sv.usd)), good),
    ]));
    if let Some(p) = sv.percent_of_would_be(app.total().cost_usd) {
        out.push(Line::from(Span::styled(
            format!("{} of what it would have cost", percent(p)),
            muted,
        )));
    }
    // The same savings per interval, newest first (whatever fits the popup).
    let buckets = app.savings_by_bucket();
    if !buckets.is_empty() {
        out.push(Line::raw(""));
        out.push(Line::from(Span::styled(
            format!("per {}", interval_name(&app.query)),
            muted,
        )));
        for b in buckets.iter().rev().take(MAX_INTERVALS) {
            out.push(Line::from(vec![
                Span::styled(format!("{:<17}", b.label), text),
                Span::styled(format!("{:>9} tok  ", compact(b.savings.tokens)), text),
                Span::styled(format!("{:>10}", app.currency.format(b.savings.usd)), good),
            ]));
        }
    }
    out
}

/// Most intervals the popup lists (newest first).
pub const MAX_INTERVALS: usize = 10;

/// `minute`, `hour`, `day` or `month`: what the savings list is grouped by.
pub fn interval_name(query: &tokenburn_core::Query) -> &'static str {
    match tokenburn_core::chart_bucket(query) {
        tokenburn_core::Bucket::Minute => "minute",
        tokenburn_core::Bucket::Hour => "hour",
        tokenburn_core::Bucket::Day => "day",
        _ => "month",
    }
}

/// The panel under the charts, drawn while the metric is `saved`. It takes the
/// height it is given; the per-interval list is cut off at the bottom.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = app.theme;
    let block = Block::default()
        .title(Line::from(Span::styled(
            " 💰 Saved ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        )))
        .title_bottom(Line::from(Span::styled(
            " price: input_price_per_mtok in settings.json ",
            Style::default().fg(t.muted),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border));
    frame.render_widget(Paragraph::new(lines(app)).block(block), area);
}

/// Rows the panel would like: its text plus the border.
pub fn wanted_height(app: &App) -> u16 {
    (lines(app).len() as u16 + 2).min(16)
}
