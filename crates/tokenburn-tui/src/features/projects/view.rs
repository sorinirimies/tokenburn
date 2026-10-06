use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;
use tokenburn_core::utils::fmt::thousands;

use crate::app::App;

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let th = app.theme;
    let border = Style::default().fg(th.border);
    if app.projects.is_empty() {
        frame.render_widget(
            Paragraph::new("No data.")
                .style(Style::default().fg(th.muted))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border)
                        .title(" Projects "),
                ),
            area,
        );
        return;
    }

    let rows: Vec<Row> = app
        .projects
        .iter()
        .map(|(name, s)| {
            Row::new([
                Cell::from(name.clone())
                    .style(Style::default().fg(th.tool(name.split(':').next().unwrap_or("")))),
                Cell::from(thousands(s.turns)),
                Cell::from(thousands(s.total_tokens)),
                Cell::from(app.currency.format(s.cost_usd)).style(Style::default().fg(th.success)),
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
        Row::new(["tool:project/model", "turns", "total", "cost"])
            .style(Style::default().fg(th.accent).add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border)
            .title(" Projects / models (by cost) "),
    )
    .row_highlight_style(Style::default().bg(th.selection))
    .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, area, &mut app.table_state);
}
