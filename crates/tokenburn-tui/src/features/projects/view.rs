use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;
use tokenburn_core::utils::fmt::thousands;
use tokenburn_core::LayoutClass;

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

    // Narrow terminals have no room for the turn count.
    let narrow = LayoutClass::from_cols(area.width).is_narrow();

    let rows: Vec<Row> = app
        .projects
        .iter()
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
        )
        .row_highlight_style(Style::default().bg(th.selection))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, area, &mut app.table_state);
}
