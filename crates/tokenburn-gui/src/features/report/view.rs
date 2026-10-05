//! Aggregated report table, with a share-of-tokens bar per line.

use iced::alignment::Horizontal;
use iced::widget::{column, container, progress_bar, row, scrollable, text};
use iced::{Element, Length};
use tokenburn_core::utils::fmt::{thousands, usd};

use crate::message::Message;
use crate::state::TokenBurn;

const NUMERIC: [&str; 6] = [
    "turns",
    "input",
    "output",
    "cache_read",
    "cache_write",
    "total",
];

pub fn view(state: &TokenBurn) -> Element<'_, Message> {
    if state.report.is_empty() {
        return container(
            text(if state.is_loading {
                "Loading…".to_string()
            } else {
                format!(
                    "No tokenburn results found for tool={}, window={}.",
                    state.query.tool, state.query.window
                )
            })
            .style(text::secondary),
        )
        .center(Length::Fill)
        .into();
    }

    let bucketed = state.report.iter().any(|r| r.bucket.is_some());
    let max_tokens = state
        .report
        .iter()
        .filter(|r| r.tool != "TOTAL")
        .map(|r| r.summary.total_tokens)
        .max()
        .unwrap_or(1)
        .max(1) as f32;

    let mut header = row![].spacing(8);
    if bucketed {
        header = header.push(cell_left("bucket", 3));
    }
    header = header.push(cell_left("tool", 2));
    for h in NUMERIC {
        header = header.push(cell_right(h.to_string(), 3));
    }
    header = header
        .push(cell_right("cost".to_string(), 3))
        .push(text("share").width(Length::FillPortion(4)));
    let header = container(header.padding([6, 10])).style(container::rounded_box);

    let mut body = column![].spacing(2);
    for r in &state.report {
        let s = &r.summary;
        let is_total = r.tool == "TOTAL";
        let mut line = row![].spacing(8).align_y(iced::Alignment::Center);
        if bucketed {
            line = line.push(cell_left(r.bucket.as_deref().unwrap_or(""), 3));
        }
        let tool = text(r.tool.clone()).width(Length::FillPortion(2));
        line = line.push(if is_total {
            tool.style(text::primary)
        } else {
            tool
        });
        for n in [
            s.turns,
            s.input,
            s.output,
            s.cache_read,
            s.cache_write,
            s.total_tokens,
        ] {
            line = line.push(cell_right(thousands(n), 3));
        }
        line = line.push(
            text(usd(s.cost_usd))
                .align_x(Horizontal::Right)
                .width(Length::FillPortion(3))
                .style(text::success),
        );
        line = line.push(
            container(if is_total {
                Element::from(text(""))
            } else {
                progress_bar(0.0..=max_tokens, s.total_tokens as f32)
                    .girth(10)
                    .into()
            })
            .width(Length::FillPortion(4)),
        );
        body = body.push(container(line).padding([4, 10]));
    }

    column![header, scrollable(body).height(Length::Fill)]
        .spacing(6)
        .into()
}

fn cell_left<'a>(s: &str, portion: u16) -> iced::widget::Text<'a> {
    text(s.to_string()).width(Length::FillPortion(portion))
}

fn cell_right<'a>(s: String, portion: u16) -> iced::widget::Text<'a> {
    text(s)
        .align_x(Horizontal::Right)
        .width(Length::FillPortion(portion))
}
