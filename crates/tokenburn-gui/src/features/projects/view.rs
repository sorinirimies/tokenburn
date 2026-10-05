//! Per-project (pi) / per-model (Zed) breakdown, most expensive first.

use iced::alignment::Horizontal;
use iced::widget::{column, container, progress_bar, row, scrollable, text};
use iced::{Element, Length};
use tokenburn_core::utils::fmt::{thousands, usd};

use crate::message::Message;
use crate::state::TokenBurn;

pub fn view(state: &TokenBurn) -> Element<'_, Message> {
    if state.projects.is_empty() {
        return container(text("No data.").style(text::secondary))
            .center(Length::Fill)
            .into();
    }

    let max_cost = state
        .projects
        .iter()
        .map(|(_, s)| s.cost_usd)
        .fold(0.0_f64, f64::max)
        .max(0.0001) as f32;

    let header = container(
        row![
            text("tool:project/model").width(Length::FillPortion(6)),
            text("turns")
                .align_x(Horizontal::Right)
                .width(Length::FillPortion(2)),
            text("total tokens")
                .align_x(Horizontal::Right)
                .width(Length::FillPortion(3)),
            text("cost")
                .align_x(Horizontal::Right)
                .width(Length::FillPortion(3)),
            text("share").width(Length::FillPortion(4)),
        ]
        .spacing(8)
        .padding([6, 10]),
    )
    .style(container::rounded_box);

    let mut body = column![].spacing(2);
    for (name, s) in &state.projects {
        body = body.push(
            container(
                row![
                    text(name.clone()).width(Length::FillPortion(6)),
                    text(thousands(s.turns))
                        .align_x(Horizontal::Right)
                        .width(Length::FillPortion(2)),
                    text(thousands(s.total_tokens))
                        .align_x(Horizontal::Right)
                        .width(Length::FillPortion(3)),
                    text(usd(s.cost_usd))
                        .align_x(Horizontal::Right)
                        .width(Length::FillPortion(3))
                        .style(text::success),
                    container(progress_bar(0.0..=max_cost, s.cost_usd as f32).girth(10))
                        .width(Length::FillPortion(4)),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .padding([4, 10]),
        );
    }

    column![header, scrollable(body).height(Length::Fill)]
        .spacing(6)
        .into()
}
