//! Top toolbar: title + query pickers (the data refreshes itself).

use iced::widget::{button, column, pick_list, row, space, text};
use iced::Element;
use tokenburn_core::{Bucket, LayoutClass, Metric, ToolFilter, Window};

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
    let mut pickers: Vec<Element<'_, Message>> = vec![
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
    // The chart metric (tokens / cost / saved) comes first in the toolbar,
    // ahead of the other filters; it only matters on the data views.
    if state.view.is_data() {
        pickers.insert(
            0,
            pick_list(
                &Metric::ALL[..],
                Some(state.chart_metric),
                Message::MetricSelected,
            )
            .into(),
        );
    }

    let menu = menu(state);
    if layout.is_wide() {
        // One toolbar: the title on the left, the filters, then the menu on the right.
        row![title, space::horizontal(), text(spin).size(18)]
            .extend(pickers)
            .push(menu)
            .spacing(10)
            .align_y(iced::Alignment::Center)
            .into()
    } else {
        // Title and menu on one line; the filters wrap underneath as far as they must.
        column![
            row![title, space::horizontal(), text(spin).size(18), menu]
                .spacing(10)
                .align_y(iced::Alignment::Center),
            row(pickers).spacing(8).wrap(),
        ]
        .spacing(8)
        .into()
    }
}

/// The top-right menu: Settings and About. The open one is highlighted and pressing
/// it again returns to the report.
fn menu(state: &TokenBurn) -> Element<'_, Message> {
    let item = |view: View, label: &'static str| {
        button(text(label).size(13))
            .padding([4, 10])
            .style(if state.view == view {
                button::primary
            } else {
                button::secondary
            })
            .on_press(Message::ViewSelected(view.toggled(state.view)))
    };
    row![item(View::Settings, "Settings"), item(View::About, "About")]
        .spacing(6)
        .into()
}
