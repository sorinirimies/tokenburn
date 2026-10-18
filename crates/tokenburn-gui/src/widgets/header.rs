//! Top toolbar: title + query pickers (the data refreshes itself).

use iced::widget::{button, column, pick_list, row, space, text};
use iced::Element;
use tokenburn_core::{Bucket, LayoutClass, Metric, ToolFilter, Window};

use crate::features::currency::CurrencyChoice;
use crate::features::theme::ThemeChoice;
use crate::message::Message;
use crate::state::{TokenBurn, View};

/// The caption above each drop-down, in toolbar order (the first only on data views).
pub const METRIC_CAPTION: &str = "Chart shows";
pub const FILTER_CAPTIONS: [&str; 5] = ["Tool", "Group by", "Period", "Currency", "Theme"];

/// Every caption the toolbar shows.
pub fn captions(data_view: bool) -> Vec<&'static str> {
    let mut out = Vec::new();
    if data_view {
        out.push(METRIC_CAPTION);
    }
    out.extend(FILTER_CAPTIONS);
    out
}

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
        labelled(
            FILTER_CAPTIONS[0],
            pick_list(
                &ToolFilter::ALL[..],
                Some(state.query.tool),
                Message::ToolSelected,
            ),
        ),
        labelled(
            FILTER_CAPTIONS[1],
            pick_list(
                &Bucket::ALL[..],
                Some(state.query.bucket),
                Message::BucketSelected,
            ),
        ),
        labelled(
            FILTER_CAPTIONS[2],
            pick_list(
                &Window::ALL[..],
                Some(state.query.window),
                Message::WindowSelected,
            ),
        ),
        labelled(
            FILTER_CAPTIONS[3],
            pick_list(
                CurrencyChoice::all(),
                Some(CurrencyChoice::of(state.currency.code)),
                Message::CurrencySelected,
            )
            .placeholder("Currency"),
        ),
        labelled(
            FILTER_CAPTIONS[4],
            pick_list(
                ThemeChoice::all(),
                Some(ThemeChoice::selected(
                    state.theme_index,
                    state.follow_system,
                )),
                Message::ThemeSelected,
            )
            .placeholder("Theme"),
        ),
    ];
    // The chart metric (tokens / cost / saved) comes first in the toolbar,
    // ahead of the other filters; it only matters on the data views.
    if state.view.is_data() {
        pickers.insert(
            0,
            labelled(
                METRIC_CAPTION,
                pick_list(
                    &Metric::ALL[..],
                    Some(state.chart_metric),
                    Message::MetricSelected,
                ),
            ),
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
                crate::features::theme::primary_button
            } else {
                crate::features::theme::action_button
            })
            .on_press(Message::ViewSelected(view.toggled(state.view)))
    };
    row![item(View::Settings, "Settings"), item(View::About, "About")]
        .spacing(6)
        .into()
}

/// A drop-down with a small caption above it, so it says what it chooses.
fn labelled<'a>(caption: &'a str, picker: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![text(caption).size(11).style(text::secondary), picker.into()]
        .spacing(2)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_drop_down_has_a_short_distinct_caption() {
        let all = captions(true);
        assert_eq!(all.len(), 6, "metric plus five filters");
        assert_eq!(all[0], "Chart shows", "the metric comes first");
        for c in &all {
            assert!(!c.is_empty() && c.len() <= 12, "'{c}' must be short to fit");
        }
        let mut sorted = all.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), all.len(), "no two drop-downs share a caption");
    }

    #[test]
    fn the_metric_caption_only_shows_on_data_views() {
        assert!(captions(true).contains(&METRIC_CAPTION));
        assert!(!captions(false).contains(&METRIC_CAPTION));
        assert_eq!(captions(false), FILTER_CAPTIONS);
    }

    #[test]
    fn the_toolbar_builds_with_captions_in_every_layout_and_view() {
        use tokenburn_core::LayoutClass;
        for view in [View::Report, View::Settings, View::About] {
            for layout in [LayoutClass::Narrow, LayoutClass::Medium, LayoutClass::Wide] {
                let mut app = TokenBurn::default();
                app.view = view;
                let _ = super::view(&app, layout);
            }
        }
    }
}
