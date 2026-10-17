use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;
use tokenburn_core::utils::fmt::thousands;
use tokenburn_core::LayoutClass;

use crate::app::App;

/// Whether the breakdown belongs under the report: only when it says something the
/// tool rows do not (a tool with several projects or models).
pub fn is_shown(app: &App) -> bool {
    tokenburn_core::projects_add_detail(&app.report, &app.projects)
}

/// Rows the section would like: header, border and up to [`MAX_ROWS`] projects.
pub fn wanted_height(app: &App) -> u16 {
    (app.projects.len().min(MAX_ROWS) as u16) + 3
}

/// Most projects listed (the most expensive ones).
pub const MAX_ROWS: usize = 6;

/// The projects / models table, drawn under the report when [`is_shown`].
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let th = app.theme;
    let border = Style::default().fg(th.border);
    // Narrow terminals have no room for the turn count.
    let narrow = LayoutClass::from_cols(area.width).is_narrow();

    let rows: Vec<Row> = app
        .projects
        .iter()
        .take(MAX_ROWS)
        .map(|(name, s)| {
            let mut cells = vec![Cell::from(name.clone())
                .style(Style::default().fg(th.tool(name.split(':').next().unwrap_or(""))))];
            if !narrow {
                cells.push(Cell::from(thousands(s.turns)));
            }
            cells.push(Cell::from(thousands(s.total_tokens)));
            cells.push(
                Cell::from(app.currency.format(s.cost_usd)).style(Style::default().fg(th.success)),
            );
            Row::new(cells)
        })
        .collect();

    let (widths, head): (Vec<Constraint>, Vec<&str>) = if narrow {
        (
            vec![
                Constraint::Min(12),
                Constraint::Length(12),
                Constraint::Length(10),
            ],
            vec!["tool:project/model", "total", "cost"],
        )
    } else {
        (
            vec![
                Constraint::Min(30),
                Constraint::Length(8),
                Constraint::Length(16),
                Constraint::Length(12),
            ],
            vec!["tool:project/model", "turns", "total", "cost"],
        )
    };

    let table = Table::new(rows, widths)
        .header(Row::new(head).style(Style::default().fg(th.accent).add_modifier(Modifier::BOLD)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border)
                .title(" Projects / models (by cost) "),
        );

    frame.render_widget(table, area);
}
