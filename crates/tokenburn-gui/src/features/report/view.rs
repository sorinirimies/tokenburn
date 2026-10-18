//! Aggregated report table, with a share-of-tokens bar per line.

use iced::alignment::Horizontal;
use iced::widget::{button, column, container, progress_bar, row, scrollable, text};
use iced::{Element, Length};
use tokenburn_core::utils::fmt::thousands;

use tokenburn_core::{LayoutClass, Metric};

use crate::features::theme::chart_colors;
use crate::message::Message;
use crate::state::TokenBurn;
use crate::widgets::charts::{donut, stacked_bars};
use crate::widgets::savings;

/// A numeric column of the table.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Col {
    Turns,
    Input,
    Output,
    CacheRead,
    CacheWrite,
    Total,
}

impl Col {
    fn title(self) -> &'static str {
        match self {
            Col::Turns => "turns",
            Col::Input => "input",
            Col::Output => "output",
            Col::CacheRead => "cache_read",
            Col::CacheWrite => "cache_write",
            Col::Total => "total",
        }
    }

    fn value(self, s: &tokenburn_core::Summary) -> u64 {
        match self {
            Col::Turns => s.turns,
            Col::Input => s.input,
            Col::Output => s.output,
            Col::CacheRead => s.cache_read,
            Col::CacheWrite => s.cache_write,
            Col::Total => s.total_tokens,
        }
    }
}

/// The numeric columns shown at `layout`: all of them when wide, no cache columns
/// when medium, only the total when narrow (plus the always-present cost).
fn columns(layout: LayoutClass) -> &'static [Col] {
    match layout {
        LayoutClass::Wide => &[
            Col::Turns,
            Col::Input,
            Col::Output,
            Col::CacheRead,
            Col::CacheWrite,
            Col::Total,
        ],
        LayoutClass::Medium => &[Col::Turns, Col::Input, Col::Output, Col::Total],
        LayoutClass::Narrow => &[Col::Total],
    }
}

/// Whether the page as a whole scrolls (cards, charts and tables together) rather
/// than just the table: when stacked, or when the saved / projects sections add height.
pub fn page_scrolls(state: &TokenBurn, layout: LayoutClass) -> bool {
    !layout.is_wide()
        || savings::view(state, layout).is_some()
        || crate::features::projects::view::is_shown(state)
}

pub fn view(state: &TokenBurn, layout: LayoutClass) -> Element<'_, Message> {
    if state.report.is_empty() {
        let message = text(if state.is_loading {
            "Loading…".to_string()
        } else {
            format!(
                "No tokenburn results found for tool={}, window={}.",
                state.query.tool, state.query.window
            )
        })
        .style(text::secondary);
        // The page scrolls when not wide, and a scrollable's content cannot fill it.
        return if layout.is_wide() {
            container(message).center(Length::Fill).into()
        } else {
            container(message).padding(30).center_x(Length::Fill).into()
        };
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
    for col in columns(layout) {
        header = header.push(cell_right(col.title().to_string(), 3));
    }
    if !layout.is_narrow() {
        header = header.push(text("share").width(Length::FillPortion(4)));
    }
    header = header.push(cell_right("cost".to_string(), 3));
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
        for col in columns(layout) {
            line = line.push(cell_right(thousands(col.value(s)), 3));
        }
        if !layout.is_narrow() {
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
        }
        line = line.push(
            text(state.currency.format(s.cost_usd))
                .align_x(Horizontal::Right)
                .width(Length::FillPortion(3))
                .style(text::success),
        );
        body = body.push(container(line).padding([4, 10]));
    }

    // The saved breakdown goes below the charts, never beside or inside them, so
    // the donut keeps its size; the projects breakdown goes under the table. While
    // either shows, the whole page scrolls.
    let saved = savings::view(state, layout);
    let projects = crate::features::projects::view::view(state, layout);
    if layout.is_wide() && saved.is_none() && projects.is_none() {
        // The charts stay put; only the table scrolls.
        column![
            charts(state, layout),
            header,
            scrollable(body).height(Length::Fill)
        ]
        .spacing(10)
        .into()
    } else {
        // Stacked charts are tall: the caller scrolls the whole page (cards, charts and
        // table together) instead of squeezing the table.
        let mut page = column![charts(state, layout)].spacing(10);
        if let Some(saved) = saved {
            page = page.push(saved);
        }
        page = page.push(header).push(body);
        if let Some(projects) = projects {
            page = page.push(projects);
        }
        page.into()
    }
}

fn cell_left<'a>(s: &str, portion: u16) -> iced::widget::Text<'a> {
    text(s.to_string()).width(Length::FillPortion(portion))
}

fn cell_right<'a>(s: String, portion: u16) -> iced::widget::Text<'a> {
    text(s)
        .align_x(Horizontal::Right)
        .width(Length::FillPortion(portion))
}

/// The bar chart's title for `metric`.
pub fn chart_title(metric: Metric) -> &'static str {
    match metric {
        Metric::Tokens => "Tokens over time",
        Metric::Cost => "Cost over time",
        Metric::Saved => "Saved over time (cache + compaction)",
    }
}

/// Stacked bars over time (left) and the share donut (right).
fn charts(state: &TokenBurn, layout: LayoutClass) -> Element<'_, Message> {
    let chart_height = if layout.is_narrow() { 170.0 } else { 200.0 };
    let colors = chart_colors(state.theme_index);
    let metric = state.chart_metric;
    let title = chart_title(metric);

    let bars = column![
        row![text(title).size(13).style(text::secondary),]
            .align_y(iced::Alignment::Center)
            .height(Length::Fixed(34.0)),
        stacked_bars(
            state.series.in_currency(&state.currency),
            metric,
            state.currency,
            colors,
            state.selection.bucket.as_deref(),
            (state.anim, state.sel_anim),
            chart_height
        ),
    ]
    .spacing(4)
    .width(if layout.is_wide() {
        Length::FillPortion(3)
    } else {
        Length::Fill
    });

    let share = column![
        row![text(format!("Share by project · {}", state.pie.metric))
            .size(13)
            .style(text::secondary)]
        .align_y(iced::Alignment::Center)
        .height(Length::Fixed(34.0)),
        donut(
            state.pie.in_currency(&state.currency),
            state.currency,
            colors,
            state.selection.project.as_deref(),
            (state.anim, state.sel_anim),
            chart_height
        ),
    ]
    .spacing(4)
    .width(if layout.is_wide() {
        Length::FillPortion(2)
    } else {
        Length::Fill
    });

    // Side by side when there is room, stacked otherwise.
    let both: Element<'_, Message> = if layout.is_wide() {
        row![bars, share].spacing(16).into()
    } else {
        column![bars, share].spacing(8).into()
    };
    let mut panel = column![both].spacing(8);
    if !state.selection.is_empty() {
        let mut chips = row![text("Showing").size(12).style(text::secondary)]
            .spacing(8)
            .align_y(iced::Alignment::Center);
        for chip in state.selection.describe() {
            chips = chips.push(
                container(text(chip).size(12))
                    .padding([2, 8])
                    .style(container::rounded_box),
            );
        }
        chips = chips.push(
            button(text("✕ clear").size(12))
                .padding([2, 8])
                .style(crate::features::theme::action_button)
                .on_press(Message::ClearSelection),
        );
        chips = chips.push(
            text("click a bar or slice again to deselect")
                .size(11)
                .style(text::secondary),
        );
        panel = panel.push(chips);
    } else {
        panel = panel.push(
            text("click a bar or a slice to drill down")
                .size(11)
                .style(text::secondary),
        );
    }

    container(panel)
        .padding(10)
        .style(container::bordered_box)
        .into()
}
