//! Limit progress bars and the alert banner, between the toolbar and the report.

use iced::widget::{button, column, container, progress_bar, row, space, text};
use iced::{Element, Length};
use tokenburn_core::{LayoutClass, Level, LimitStatus};

use crate::message::Message;
use crate::state::TokenBurn;

/// The text of one limit: `daily  $12.40 / $20.00  62%`.
pub fn caption(state: &TokenBurn, s: &LimitStatus) -> String {
    format!(
        "{} {} / {} ({}%)",
        s.period.label(),
        state.currency.format(s.spent_usd),
        state.currency.format(s.limit_usd),
        s.percent()
    )
}

fn one<'a>(state: &'a TokenBurn, s: &LimitStatus) -> Element<'a, Message> {
    let bar = progress_bar(0.0..=1.0, s.ratio().clamp(0.0, 1.0) as f32)
        .girth(8)
        .style(match s.level {
            Level::Ok => progress_bar::success,
            Level::Warn => progress_bar::warning,
            Level::Over => progress_bar::danger,
        });
    let label = text(caption(state, s)).size(12);
    let label = match s.level {
        Level::Ok => label.style(text::secondary),
        Level::Warn => label.style(text::warning),
        Level::Over => label.style(text::danger),
    };
    column![label, bar].spacing(3).width(Length::Fill).into()
}

/// The banner for the latest alert, then one bar per limit. `None` when there is
/// nothing to show.
pub fn view(state: &TokenBurn, layout: LayoutClass) -> Option<Element<'_, Message>> {
    if state.limit_statuses.is_empty() && state.alert.is_none() {
        return None;
    }
    let mut out = column![].spacing(6);
    if let Some(a) = &state.alert {
        out = out.push(
            container(
                row![
                    text(a.clone()).style(text::warning),
                    space::horizontal(),
                    button(text("Dismiss").size(12))
                        .style(crate::features::theme::action_button)
                        .on_press(Message::DismissAlert),
                ]
                .align_y(iced::Alignment::Center),
            )
            .padding(8)
            .width(Length::Fill)
            .style(container::bordered_box),
        );
    }
    if !state.limit_statuses.is_empty() {
        let bars: Vec<Element<'_, Message>> =
            state.limit_statuses.iter().map(|s| one(state, s)).collect();
        let bars: Element<'_, Message> = if layout.is_narrow() {
            column(bars).spacing(6).into()
        } else {
            row(bars).spacing(16).into()
        };
        out = out.push(bars);
    }
    Some(out.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokenburn_core::{Alert, Period};

    fn status(period: Period, spent: f64, level: Level) -> LimitStatus {
        LimitStatus {
            period,
            limit_usd: 20.0,
            spent_usd: spent,
            level,
        }
    }

    #[test]
    fn nothing_shows_without_limits_or_an_alert() {
        let app = TokenBurn::new();
        assert!(view(&app, LayoutClass::Wide).is_none());
    }

    #[test]
    fn captions_show_amounts_and_percent_in_the_display_currency() {
        let mut app = TokenBurn::new();
        let s = status(Period::Day, 12.4, Level::Ok);
        assert_eq!(caption(&app, &s), "daily $12.40 / $20.00 (62%)");
        app.set_currency(tokenburn_core::features::currency::index_of("EUR"));
        assert!(caption(&app, &s).contains('€'));
    }

    #[test]
    fn every_level_and_layout_builds_with_and_without_the_banner() {
        let mut app = TokenBurn::new();
        app.limit_statuses = vec![
            status(Period::Day, 5.0, Level::Ok),
            status(Period::Week, 18.0, Level::Warn),
            status(Period::Month, 99.0, Level::Over),
        ];
        for layout in [LayoutClass::Wide, LayoutClass::Medium, LayoutClass::Narrow] {
            assert!(view(&app, layout).is_some());
        }
        app.apply_limit_check(tokenburn_core::LimitCheck {
            limits: app.limits.clone(),
            statuses: app.limit_statuses.clone(),
            alerts: vec![Alert {
                period: Period::Month,
                level: Level::Over,
                spent_usd: 99.0,
                limit_usd: 20.0,
            }],
        });
        let banner = app.alert.clone().unwrap();
        assert!(
            banner.contains("monthly limit exceeded") && banner.contains("$99.00"),
            "{banner}"
        );
        assert!(view(&app, LayoutClass::Wide).is_some());
    }
}
