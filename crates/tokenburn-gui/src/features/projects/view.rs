//! Per-project / per-model breakdown, most expensive first.

use iced::alignment::Horizontal;
use iced::widget::{column, container, progress_bar, row, text};
use iced::{Element, Length};
use tokenburn_core::utils::fmt::thousands;
use tokenburn_core::LayoutClass;

use crate::message::Message;
use crate::state::TokenBurn;

/// Whether the breakdown belongs under the report: only when it says something the
/// report's tool rows do not (a tool with several projects or models).
pub fn is_shown(state: &TokenBurn) -> bool {
    tokenburn_core::projects_add_detail(&state.report, &state.projects)
}

/// The breakdown as a titled section, for the bottom of the report. `None` when it
/// would only repeat the report.
pub fn view(state: &TokenBurn, layout: LayoutClass) -> Option<Element<'_, Message>> {
    if !is_shown(state) {
        return None;
    }
    let narrow = layout.is_narrow();

    let max_cost = state
        .projects
        .iter()
        .map(|(_, s)| s.cost_usd)
        .fold(0.0_f64, f64::max)
        .max(0.0001) as f32;

    let mut head = row![text("tool:project/model").width(Length::FillPortion(6))];
    if !narrow {
        head = head.push(
            text("turns")
                .align_x(Horizontal::Right)
                .width(Length::FillPortion(2)),
        );
    }
    head = head
        .push(
            text("total tokens")
                .align_x(Horizontal::Right)
                .width(Length::FillPortion(3)),
        )
        .push(
            text("cost")
                .align_x(Horizontal::Right)
                .width(Length::FillPortion(3)),
        );
    if !narrow {
        head = head.push(text("share").width(Length::FillPortion(4)));
    }
    let header = container(head.spacing(8).padding([6, 10])).style(container::rounded_box);

    let mut body = column![].spacing(2);
    for (name, s) in &state.projects {
        let mut line = row![text(name.clone()).width(Length::FillPortion(6))];
        if !narrow {
            line = line.push(
                text(thousands(s.turns))
                    .align_x(Horizontal::Right)
                    .width(Length::FillPortion(2)),
            );
        }
        line = line
            .push(
                text(thousands(s.total_tokens))
                    .align_x(Horizontal::Right)
                    .width(Length::FillPortion(3)),
            )
            .push(
                text(state.currency.format(s.cost_usd))
                    .align_x(Horizontal::Right)
                    .width(Length::FillPortion(3))
                    .style(text::success),
            );
        if !narrow {
            line = line.push(
                container(progress_bar(0.0..=max_cost, s.cost_usd as f32).girth(10))
                    .width(Length::FillPortion(4)),
            );
        }
        body =
            body.push(container(line.spacing(8).align_y(iced::Alignment::Center)).padding([4, 10]));
    }

    Some(
        column![
            text("Projects / models (by cost)")
                .size(13)
                .style(text::secondary),
            header,
            body
        ]
        .spacing(6)
        .into(),
    )
}
