//! Top toolbar: title + query pickers + refresh.

use iced::widget::{button, pick_list, row, space, text};
use iced::{Element, Theme};
use tokenburn_core::{Bucket, ToolFilter, Window};

use crate::message::Message;
use crate::state::{TokenBurn, View};

const SPINNER: [&str; 4] = ["◐", "◓", "◑", "◒"];

pub fn view(state: &TokenBurn) -> Element<'_, Message> {
    let spin = if state.is_loading {
        SPINNER[(state.tick / 2) as usize % SPINNER.len()]
    } else {
        ""
    };

    row![
        text("🔥 TokenBurn").size(22).style(text::primary),
        space::horizontal(),
        text(spin).size(18),
        pick_list(&View::ALL[..], Some(state.view), Message::ViewSelected),
        pick_list(
            &ToolFilter::ALL[..],
            Some(state.query.tool),
            Message::ToolSelected
        ),
        pick_list(
            &Bucket::ALL[..],
            Some(state.query.bucket),
            Message::BucketSelected
        ),
        pick_list(
            &Window::ALL[..],
            Some(state.query.window),
            Message::WindowSelected
        ),
        pick_list(
            Theme::ALL,
            Some(state.theme.clone()),
            Message::ThemeSelected
        ),
        button("Refresh").on_press(Message::Refresh),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center)
    .into()
}
