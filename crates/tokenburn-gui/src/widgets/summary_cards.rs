//! Row of headline totals.

use iced::widget::{column, container, row, text};
use iced::{Element, Length};
use tokenburn_core::LayoutClass;

use crate::message::Message;
use crate::state::TokenBurn;

/// Wide: one stretching row. Otherwise the cards keep a fixed size and **wrap** onto
/// as many rows as the window width allows.
pub fn view(state: &TokenBurn, layout: LayoutClass) -> Element<'_, Message> {
    let width = if layout.is_wide() {
        Length::Fill
    } else if layout.is_narrow() {
        Length::Fixed(112.0)
    } else {
        Length::Fixed(150.0)
    };
    // Which figures show follows the chart metric (tokens / cost / saved).
    let cards: Vec<_> = tokenburn_core::cards(
        state.chart_metric,
        &state.total(),
        &state.savings,
        &state.currency,
    )
    .into_iter()
    .map(|c| card(c.label, c.value, width))
    .collect();
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokenburn_core::Metric;

    #[test]
    fn the_cards_build_for_every_metric_and_layout() {
        let mut app = TokenBurn::default();
        for metric in Metric::ALL {
            app.chart_metric = metric;
            for layout in [LayoutClass::Wide, LayoutClass::Medium, LayoutClass::Narrow] {
                let _ = view(&app, layout);
            }
        }
    }
}
