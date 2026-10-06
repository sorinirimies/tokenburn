//! Title bar with the grand total and the active query.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use tokenburn_core::utils::fmt::compact;

use crate::app::App;
use crate::features::theme::UiTheme;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = app.total();
    let q = app.query;
    let th = app.theme;

    let title = Line::from(vec![
        Span::styled(
            " 🔥 TokenBurn ",
            Style::default().fg(th.warning).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        chip(th, "tool", q.tool.as_str()),
        chip(th, "view", q.bucket.label()),
        chip(th, "window", q.window.label()),
        chip(th, "theme", tokenburn_core::THEME_NAMES[app.theme_index]),
        chip(
            th,
            "cur",
            &format!("{}{}", app.currency.code, app.currency.approx_mark()),
        ),
        filter_chips(app),
        Span::styled(
            if app.is_loading { " loading…" } else { "" },
            Style::default().fg(th.muted),
        ),
    ]);

    let totals = Line::from(vec![
        stat(th, "turns", t.turns.to_string()),
        stat(th, "total", compact(t.total_tokens)),
        stat(th, "in", compact(t.input)),
        stat(th, "out", compact(t.output)),
        stat(
            th,
            "cache r/w",
            format!("{}/{}", compact(t.cache_read), compact(t.cache_write)),
        ),
        Span::styled(
            format!(" {} ", app.currency.format(t.cost_usd)),
            Style::default()
                .fg(th.bg)
                .bg(th.success)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(th.border));
    frame.render_widget(Paragraph::new(vec![title, totals]).block(block), area);
}

fn chip(th: UiTheme, label: &str, value: &str) -> Span<'static> {
    Span::styled(format!(" {label}:{value} "), Style::default().fg(th.accent))
}

fn stat(th: UiTheme, label: &str, value: String) -> Span<'static> {
    Span::styled(format!(" {label} {value} │"), Style::default().fg(th.text))
}

/// `filter:"text"  tools:3/8  412/3,029` — empty when nothing is filtered.
fn filter_chips(app: &App) -> Span<'static> {
    let th = app.theme;
    if app.filter.is_empty() {
        return Span::raw("");
    }
    let mut parts = Vec::new();
    if !app.filter.text.trim().is_empty() {
        parts.push(format!("filter:\"{}\"", app.filter.text.trim()));
    }
    if app.filter.tools.is_some() {
        parts.push(format!(
            "tools:{}/{}",
            app.filter.tool_count(),
            tokenburn_core::Tool::ALL.len()
        ));
    }
    parts.push(format!("{}/{} rows", app.rows.len(), app.all_rows.len()));
    Span::styled(
        format!(" ⌕ {} ", parts.join("  ")),
        Style::default()
            .fg(th.bg)
            .bg(th.warning)
            .add_modifier(Modifier::BOLD),
    )
}
