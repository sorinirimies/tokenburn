//! Title bar with the grand total and the active query.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use tokenburn_core::utils::fmt::{compact, usd};

use crate::app::App;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = app.total();
    let q = app.query;

    let title = Line::from(vec![
        Span::styled(
            " 🔥 TokenBurn ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        chip("tool", q.tool.as_str()),
        chip("view", q.bucket.label()),
        chip("window", q.window.label()),
        Span::raw(if app.is_loading { " loading…" } else { "" }),
    ]);

    let totals = Line::from(vec![
        stat("turns", t.turns.to_string()),
        stat("total", compact(t.total_tokens)),
        stat("in", compact(t.input)),
        stat("out", compact(t.output)),
        stat(
            "cache r/w",
            format!("{}/{}", compact(t.cache_read), compact(t.cache_write)),
        ),
        Span::styled(
            format!(" {} ", usd(t.cost_usd)),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    frame.render_widget(Paragraph::new(vec![title, totals]).block(block), area);
}

fn chip(label: &str, value: &str) -> Span<'static> {
    Span::styled(
        format!(" {label}:{value} "),
        Style::default().fg(Color::Cyan),
    )
}

fn stat(label: &str, value: String) -> Span<'static> {
    Span::styled(
        format!(" {label} {value} │"),
        Style::default().fg(Color::White),
    )
}
