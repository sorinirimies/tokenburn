//! Row of headline totals.

use iced::widget::{column, container, row, text};
use iced::{Element, Length};
use tokenburn_core::utils::fmt::compact;

use crate::message::Message;
use crate::state::TokenBurn;

pub fn view(state: &TokenBurn) -> Element<'_, Message> {
    let t = state.total();
    // "cost ≈" when the exchange rate is a built-in estimate.
    let cost_label = format!("cost{}", state.currency.approx_mark());
    row![
        card("turns", t.turns.to_string()),
        card("total tokens", compact(t.total_tokens)),
        card("input", compact(t.input)),
        card("output", compact(t.output)),
        card("cache read", compact(t.cache_read)),
        card("cache write", compact(t.cache_write)),
        card(cost_label, state.currency.format(t.cost_usd)),
    ]
    .spacing(10)
    .into()
}

fn card<'a>(label: impl text::IntoFragment<'a>, value: String) -> Element<'a, Message> {
    container(
        column![
            text(label).size(12).style(text::secondary),
            text(value).size(20),
        ]
        .spacing(2),
    )
    .padding(10)
    .width(Length::Fill)
    .style(container::bordered_box)
    .into()
}
