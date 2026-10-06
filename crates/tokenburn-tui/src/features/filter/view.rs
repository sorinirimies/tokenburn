//! The per-tool checkbox popup.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use tokenburn_core::utils::fmt::thousands;
use tokenburn_core::Tool;
use tui_checkbox::Checkbox;

use crate::app::App;
use crate::features::options::view::centered;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = app.theme;
    let h = Tool::ALL.len() as u16 + 4;
    let popup = centered(area, 46, h);
    frame.render_widget(Clear, popup);

    let shown = app.filter.tool_count();
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled("⌕ ", Style::default().fg(t.accent)),
            Span::styled(
                "Tools",
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" · {shown}/{} shown ", Tool::ALL.len()),
                Style::default().fg(t.muted),
            ),
        ]))
        .title_bottom(Line::from(Span::styled(
            " ␣ toggle · a all · n none · x clear all filters ",
            Style::default().fg(t.muted),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent))
        .style(Style::default().bg(t.surface).fg(t.text));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::vertical(vec![Constraint::Length(1); Tool::ALL.len()]).split(inner);
    for (i, tool) in Tool::ALL.iter().enumerate() {
        let selected = i == app.tool_cursor;
        let on = app.filter.allows(*tool);
        let [mark, body, count] = Layout::horizontal([
            Constraint::Length(2),
            Constraint::Min(10),
            Constraint::Length(10),
        ])
        .areas(rows[i]);

        frame.render_widget(
            Paragraph::new(if selected { "▶ " } else { "  " }).style(Style::default().fg(t.accent)),
            mark,
        );
        let label_style = Style::default()
            .fg(if on { t.tool(tool.as_str()) } else { t.muted })
            .add_modifier(if selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            });
        frame.render_widget(
            Checkbox::new(tool.display_name(), on)
                .checked_symbol("☑")
                .unchecked_symbol("☐")
                .checkbox_style(Style::default().fg(if on { t.success } else { t.muted }))
                .label_style(label_style),
            body,
        );
        frame.render_widget(
            Paragraph::new(thousands(app.tool_counts[tool.series_index()] as u64))
                .alignment(ratatui::layout::Alignment::Right)
                .style(Style::default().fg(t.muted)),
            count,
        );
    }
}
