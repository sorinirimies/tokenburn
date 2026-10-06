//! The currency picker popup.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};
use ratatui::Frame;
use tokenburn_core::CURRENCIES;

use crate::app::App;
use crate::features::options::view::centered;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = app.theme;
    let popup = centered(area, 56, area.height.saturating_sub(4).clamp(8, 26));
    frame.render_widget(Clear, popup);

    let current = app.currency.code;
    let items: Vec<ListItem> = CURRENCIES
        .iter()
        .map(|c| {
            // The rate that is in use (the user's own for the current one, if set).
            let (rate, own) = if c.code == current {
                (app.currency.per_usd, app.currency.custom_rate)
            } else {
                (c.per_usd, false)
            };
            let rate_text = if c.code == "USD" {
                "—".to_string()
            } else {
                format!("{rate} per $ {}", if own { "(yours)" } else { "≈" })
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<4}", c.code),
                    if c.code == current {
                        Style::default().fg(t.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(t.text)
                    },
                ),
                Span::styled(format!("{:<22}", c.name), Style::default().fg(t.text)),
                Span::styled(rate_text, Style::default().fg(t.muted)),
            ]))
        })
        .collect();

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled("💱 ", Style::default().fg(t.accent)),
            Span::styled(
                "Currency",
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " · rates are fixed estimates  ",
                Style::default().fg(t.muted),
            ),
        ]))
        .title_bottom(Line::from(Span::styled(
            " ↑/↓ preview · Esc close · rates: settings.json ",
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

    let selected = tokenburn_core::features::currency::index_of(current);
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(list, popup, &mut state);
}
