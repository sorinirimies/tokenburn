//! The Elm `view` function.

use iced::widget::{column, container, text};
use iced::{Element, Length};

use crate::features::{projects, report};
use crate::message::Message;
use crate::state::{TokenBurn, View};
use crate::widgets::{header, summary_cards};

impl TokenBurn {
    pub fn view(&self) -> Element<'_, Message> {
        let body = match self.view {
            View::Report => report::view::view(self),
            View::Projects => projects::view::view(self),
        };

        let status: Element<'_, Message> = if let Some(e) = &self.error_message {
            text(format!("⚠ {e}")).style(text::danger).into()
        } else if let Some(w) = self.warnings.first() {
            text(format!("⚠ {w}")).style(text::warning).into()
        } else {
            text(self.status_message.clone().unwrap_or_default())
                .size(12)
                .style(text::secondary)
                .into()
        };

        container(column![header::view(self), summary_cards::view(self), body, status].spacing(12))
            .padding(16)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
