use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;
use tokenburn_core::utils::fmt::{thousands, usd};

use crate::app::App;

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let th = app.theme;
    let border = Style::default().fg(th.border);
    let bucketed = app.report.iter().any(|r| r.bucket.is_some());

    let mut head = Vec::new();
    if bucketed {
        head.push("bucket");
    }
    head.extend([
        "tool",
        "turns",
        "input",
        "output",
        "cache_read",
        "cache_write",
        "total",
        "cost",
    ]);

    let rows: Vec<Row> = app
        .report
        .iter()
        .map(|r| {
            let s = &r.summary;
            let mut cells = Vec::new();
            if bucketed {
                cells.push(Cell::from(r.bucket.clone().unwrap_or_default()));
            }
            cells.push(Cell::from(r.tool.clone()).style(Style::default().fg(
                if r.tool == "TOTAL" {
                    th.warning
                } else {
                    th.tool(&r.tool)
                },
            )));
            for n in [
                s.turns,
                s.input,
                s.output,
                s.cache_read,
                s.cache_write,
                s.total_tokens,
            ] {
                cells.push(Cell::from(thousands(n)).style(Style::default().fg(th.text)));
            }
            cells.push(Cell::from(usd(s.cost_usd)).style(Style::default().fg(th.success)));
            let mut row = Row::new(cells);
            if r.tool == "TOTAL" {
                row = row.style(Style::default().add_modifier(Modifier::BOLD).fg(th.warning));
            }
            row
        })
        .collect();

    let mut widths = Vec::new();
    if bucketed {
        widths.push(Constraint::Length(17));
    }
    widths.push(Constraint::Length(6));
    widths.extend([Constraint::Min(8); 7]);

    let table = Table::new(rows, widths)
        .header(Row::new(head).style(Style::default().fg(th.accent).add_modifier(Modifier::BOLD)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border)
                .title(format!(" Report · {} ", app.query.bucket)),
        )
        .row_highlight_style(Style::default().bg(th.selection))
        .highlight_symbol("▶ ");

    if app.report.is_empty() && !app.is_loading {
        let text = if app.all_rows.is_empty() {
            format!(
                "No tokenburn results found for tool={}, window={}.",
                app.query.tool, app.query.window
            )
        } else {
            format!(
                "The filter hides all {} rows — Esc clears it, f edits the tools.",
                app.all_rows.len()
            )
        };
        let msg = ratatui::widgets::Paragraph::new(text)
            .style(Style::default().fg(th.muted))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border)
                    .title(" Report "),
            );
        frame.render_widget(msg, area);
        return;
    }

    frame.render_stateful_widget(table, area, &mut app.table_state);
}
