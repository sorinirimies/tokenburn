//! The theme picker popup.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};
use ratatui::Frame;
use tokenburn_core::theme_by_index;

use crate::app::App;
use crate::features::options::view::centered;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = app.theme;
    let popup = centered(area, 44, area.height.saturating_sub(4).clamp(8, 24));
    frame.render_widget(Clear, popup);

    let items: Vec<ListItem> = (0..super::PICKER_LEN)
        .map(|i| {
            // A swatch of the theme's own accent + first series colours; "System"
            // previews the preset it currently resolves to.
            let preview = if i == 0 {
                theme_by_index(tokenburn_core::current_system_theme_index())
            } else {
                theme_by_index(i - 1)
            };
            let active = i == app.picker_index();
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
                    super::picker_name(i).to_string(),
                    if active {
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
                format!(" · {}/{}  ", app.picker_index() + 1, super::PICKER_LEN),
                Style::default().fg(t.muted),
            ),
        ]))
        .title_bottom(Line::from(Span::styled(
            " ↑/↓ preview · System follows OS · Esc close ",
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

    let mut state = ListState::default().with_selected(Some(app.picker_index()));
    frame.render_stateful_widget(list, popup, &mut state);
}
