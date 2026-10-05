use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;
use tokenburn_core::utils::fmt::{thousands, usd};

use crate::app::App;

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    if app.projects.is_empty() {
        frame.render_widget(
            Paragraph::new("No data.")
                .block(Block::default().borders(Borders::ALL).title(" Projects ")),
            area,
        );
        return;
    }

    let rows: Vec<Row> = app
        .projects
        .iter()
        .map(|(name, s)| {
            Row::new([
                Cell::from(name.clone()),
                Cell::from(thousands(s.turns)),
                Cell::from(thousands(s.total_tokens)),
                Cell::from(usd(s.cost_usd)).style(Style::default().fg(Color::Green)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(30),
            Constraint::Length(8),
            Constraint::Length(16),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(["tool:project/model", "turns", "total", "cost"]).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Projects / models (by cost) "),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray))
    .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, area, &mut app.table_state);
}
