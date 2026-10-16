//! Top toolbar: title + query pickers (the data refreshes itself).

use iced::widget::{column, pick_list, row, space, text};
use iced::Element;
use tokenburn_core::{Bucket, LayoutClass, ToolFilter, Window};

use crate::features::currency::CurrencyChoice;
use crate::features::theme::ThemeChoice;
use crate::message::Message;
use crate::state::{TokenBurn, View};

const SPINNER: [&str; 4] = ["◐", "◓", "◑", "◒"];

pub fn view(state: &TokenBurn, layout: LayoutClass) -> Element<'_, Message> {
    let spin = if state.is_loading {
        SPINNER[(state.tick / 2) as usize % SPINNER.len()]
    } else {
        ""
    };

    let title = row![
        text("🔥 TokenBurn").size(22).style(text::primary),
        text(tokenburn_core::appinfo::version_label())
            .size(13)
            .style(text::secondary),
    ]
    .spacing(8)
    .align_y(iced::Alignment::End);
    let pickers = [
        pick_list(&View::ALL[..], Some(state.view), Message::ViewSelected).into(),
        pick_list(
            &ToolFilter::ALL[..],
            Some(state.query.tool),
            Message::ToolSelected,
        )
        .into(),
        pick_list(
            &Bucket::ALL[..],
            Some(state.query.bucket),
            Message::BucketSelected,
        )
        .into(),
        pick_list(
            &Window::ALL[..],
            Some(state.query.window),
            Message::WindowSelected,
        )
        .into(),
        pick_list(
            CurrencyChoice::all(),
            Some(CurrencyChoice::of(state.currency.code)),
            Message::CurrencySelected,
        )
        .placeholder("Currency")
        .into(),
        pick_list(
            ThemeChoice::all(),
            Some(ThemeChoice::selected(
                state.theme_index,
                state.follow_system,
            )),
            Message::ThemeSelected,
        )
        .placeholder("Theme")
        .into(),
    ];

    if layout.is_wide() {
        // One toolbar: the title on the left, every control on the right.
        row![title, space::horizontal(), text(spin).size(18)]
            .extend(pickers)
            .spacing(10)
            .align_y(iced::Alignment::Center)
            .into()
    } else {
        // Title on its own line; the controls wrap underneath as far as they must.
        column![
            row![title, space::horizontal(), text(spin).size(18)].align_y(iced::Alignment::Center),
            row(pickers).spacing(8).wrap(),
        ]
        .spacing(8)
        .into()
    }
}
