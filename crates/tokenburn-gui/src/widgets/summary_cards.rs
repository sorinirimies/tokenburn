//! Row of headline totals.

use iced::widget::{column, container, row, text};
use iced::{Element, Length};
use tokenburn_core::utils::fmt::compact;
use tokenburn_core::LayoutClass;

use crate::message::Message;
use crate::state::TokenBurn;

/// Wide: one stretching row. Otherwise the cards keep a fixed size and **wrap** onto
/// as many rows as the window width allows.
pub fn view(state: &TokenBurn, layout: LayoutClass) -> Element<'_, Message> {
    let t = state.total();
    // "cost ≈" when the exchange rate is a built-in estimate.
    let cost_label = format!("cost{}", state.currency.approx_mark());
    let width = if layout.is_wide() {
        Length::Fill
    } else if layout.is_narrow() {
        Length::Fixed(112.0)
    } else {
        Length::Fixed(150.0)
    };
    let cards = [
        card("turns".to_string(), t.turns.to_string(), width),
        card("total tokens".to_string(), compact(t.total_tokens), width),
        card("input".to_string(), compact(t.input), width),
        card("output".to_string(), compact(t.output), width),
        card("cache read".to_string(), compact(t.cache_read), width),
        card("cache write".to_string(), compact(t.cache_write), width),
        card(cost_label, state.currency.format(t.cost_usd), width),
    ];
    let cards = row(cards).spacing(10);
    if layout.is_wide() {
        cards.into()
    } else {
        cards.wrap().into()
    }
}

fn card<'a>(label: String, value: String, width: Length) -> Element<'a, Message> {
    container(
        column![
            text(label).size(12).style(text::secondary),
            text(value).size(20),
        ]
        .spacing(2),
    )
    .padding(10)
    .width(width)
    .style(container::bordered_box)
    .into()
}
