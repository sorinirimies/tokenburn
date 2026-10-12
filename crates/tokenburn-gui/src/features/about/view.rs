//! About (version, developer, links) and Settings (what is saved, where) panels.

use iced::widget::{column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::message::Message;
use crate::state::{TokenBurn, AUTO_REFRESH};

fn line<'a>(label: &str, value: String) -> Element<'a, Message> {
    row![
        text(label.to_string())
            .style(text::secondary)
            .width(Length::Fixed(130.0)),
        text(value),
    ]
    .spacing(12)
    .into()
}

fn panel<'a>(title: String, rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    scrollable(
        container(
            column![text(title).size(20).style(text::primary)]
                .extend(rows)
                .spacing(10),
        )
        .padding(16),
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .into()
}

/// Version, developer and links.
pub fn about(_state: &TokenBurn) -> Element<'_, Message> {
    let mut rows: Vec<Element<'_, Message>> = vec![text(tokenburn_core::appinfo::TAGLINE)
        .style(text::secondary)
        .into()];
    rows.extend(
        tokenburn_core::appinfo::about_rows()
            .into_iter()
            .map(|(k, v)| line(k, v)),
    );
    panel(format!("🔥 {}", tokenburn_core::appinfo::title()), rows)
}

/// Current theme, currency and where they are stored. Both are changed with the
/// toolbar pickers and shared with the TUI and web dashboard.
pub fn settings(state: &TokenBurn) -> Element<'_, Message> {
    let rows = vec![
        line("Theme", tokenburn_core::THEME_NAMES[state.theme_index].to_string()),
        line(
            "Currency",
            format!("{}{}", state.currency.code, state.currency.approx_mark()),
        ),
        line("Auto-refresh", format!("every {} s", AUTO_REFRESH.as_secs())),
        line("Settings file", tokenburn_core::appinfo::settings_file()),
        text("Theme and currency are changed in the toolbar and shared with the TUI and web dashboard.")
            .size(12)
            .style(text::secondary)
            .into(),
    ];
    panel("⚙ Settings".to_string(), rows)
}
