use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;
use tokenburn_core::utils::fmt::thousands;
use tokenburn_core::{LayoutClass, Summary};

use crate::app::App;

/// A numeric column of the report table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Col {
    Turns,
    Input,
    Output,
    CacheRead,
    CacheWrite,
    Total,
}

impl Col {
    pub fn title(self) -> &'static str {
        match self {
            Col::Turns => "turns",
            Col::Input => "input",
            Col::Output => "output",
            Col::CacheRead => "cache_read",
            Col::CacheWrite => "cache_write",
            Col::Total => "total",
        }
    }

    pub fn value(self, s: &Summary) -> u64 {
        match self {
            Col::Turns => s.turns,
            Col::Input => s.input,
            Col::Output => s.output,
            Col::CacheRead => s.cache_read,
            Col::CacheWrite => s.cache_write,
            Col::Total => s.total_tokens,
        }
    }
}

/// The numeric columns shown at `class` (the cost column is always there).
pub fn columns(class: LayoutClass) -> &'static [Col] {
    match class {
        LayoutClass::Wide => &[
            Col::Turns,
            Col::Input,
            Col::Output,
            Col::CacheRead,
            Col::CacheWrite,
            Col::Total,
        ],
        LayoutClass::Medium => &[Col::Turns, Col::Input, Col::Output, Col::Total],
        LayoutClass::Narrow => &[Col::Total],
    }
}

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let th = app.theme;
    let border = Style::default().fg(th.border);
    let bucketed = app.report.iter().any(|r| r.bucket.is_some());

    // Which numeric columns fit: all when wide, no cache columns when medium, just the
    // total (and the cost) when narrow.
    let class = LayoutClass::from_cols(area.width);
    let cols = columns(class);

    let mut head = Vec::new();
    if bucketed {
        head.push("bucket");
    }
    head.push("tool");
    head.extend(cols.iter().map(|c| c.title()));
    head.push("cost");

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
            for col in cols {
                cells.push(Cell::from(thousands(col.value(s))).style(Style::default().fg(th.text)));
            }
            cells.push(
                Cell::from(app.currency.format(s.cost_usd)).style(Style::default().fg(th.success)),
            );
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
    widths.push(Constraint::Length(if class.is_narrow() { 8 } else { 6 }));
    widths.extend(std::iter::repeat_n(Constraint::Min(8), cols.len() + 1));

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
