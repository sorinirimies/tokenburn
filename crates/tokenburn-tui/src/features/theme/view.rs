//! The theme picker popup.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};
use ratatui::Frame;
use tokenburn_core::{theme_by_index, THEME_NAMES};

use crate::app::App;
use crate::features::options::view::centered;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = app.theme;
    let popup = centered(area, 44, area.height.saturating_sub(4).clamp(8, 24));
    frame.render_widget(Clear, popup);

    let items: Vec<ListItem> = THEME_NAMES
        .iter()
        .enumerate()
        .map(|(i, name)| {
            // A swatch of the theme's own accent + first series colours.
            let preview = theme_by_index(i);
            let swatch = |c: tokenburn_core::Rgb| {
                Span::styled(
                    "■",
                    Style::default().fg(ratatui::style::Color::Rgb(c.r, c.g, c.b)),
                )
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:>2} ", i + 1), Style::default().fg(t.muted)),
                swatch(preview.accent),
                swatch(preview.series[1]),
                swatch(preview.series[2]),
                Span::raw(" "),
                Span::styled(
                    (*name).to_string(),
                    if i == app.theme_index {
                        Style::default().fg(t.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(t.text)
                    },
                ),
            ]))
        })
        .collect();

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled("🎨 ", Style::default().fg(t.accent)),
            Span::styled(
                "Theme",
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" · {}/{}  ", app.theme_index + 1, THEME_NAMES.len()),
                Style::default().fg(t.muted),
            ),
        ]))
        .title_bottom(Line::from(Span::styled(
            " ↑/↓ preview · Esc/Enter close ",
            Style::default().fg(t.muted),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent))
        .style(Style::default().bg(t.surface).fg(t.text));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(t.selection)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default().with_selected(Some(app.theme_index));
    frame.render_stateful_widget(list, popup, &mut state);
}
