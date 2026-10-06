//! The Elm `view` function.

use iced::widget::{column, container, text};
use iced::{Element, Length};

use crate::features::{projects, report};
use crate::message::Message;
use crate::state::{TokenBurn, View};
use crate::widgets::{header, summary_cards};

impl TokenBurn {
    pub fn view(&self) -> Element<'_, Message> {
        let body = match self.view {
            View::Report => report::view::view(self),
            View::Projects => projects::view::view(self),
        };

        let status: Element<'_, Message> = if let Some(e) = &self.error_message {
            text(format!("⚠ {e}")).style(text::danger).into()
        } else if let Some(w) = self.warnings.first() {
            text(format!("⚠ {w}")).style(text::warning).into()
        } else {
            text(self.status_message.clone().unwrap_or_default())
                .size(12)
                .style(text::secondary)
                .into()
        };

        container(column![header::view(self), summary_cards::view(self), body, status].spacing(12))
            .padding(16)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokenburn_core::{Collected, Row, Tool};

    fn row(tool: Tool, project: &str, ts: &str, tokens: u64) -> Row {
        Row {
            tool,
            project: project.into(),
            id: "x".into(),
            ts: ts.parse().unwrap(),
            input: tokens,
            output: tokens / 4,
            cache_read: 3,
            cache_write: 1,
            cost: if tool == Tool::Pi { 0.5 } else { 0.0 },
        }
    }

    fn loaded() -> TokenBurn {
        let mut app = TokenBurn::new();
        app.apply(Collected {
            rows: vec![
                row(Tool::Pi, "alpha", "2026-05-01T10:00:00Z", 1000),
                row(Tool::Zed, "beta", "2026-05-02T10:00:00Z", 2000),
                row(Tool::Claude, "alpha", "2026-05-02T11:00:00Z", 500),
            ],
            warnings: vec![],
        });
        app
    }

    // Building the widget tree needs no renderer, so every view can be exercised
    // headlessly: these catch panics (slicing, empty data, bad indices) on each path.

    #[test]
    fn the_report_view_builds_with_and_without_data() {
        let _ = TokenBurn::new().view();
        let _ = loaded().view();
    }

    #[test]
    fn the_projects_view_builds() {
        let mut app = loaded();
        app.view = View::Projects;
        let _ = app.view();
        let mut empty = TokenBurn::new();
        empty.view = View::Projects;
        let _ = empty.view();
    }

    #[test]
    fn every_status_line_variant_builds() {
        let mut app = loaded();
        app.is_loading = true;
        let _ = app.view();
        app.is_loading = false;
        app.warnings = vec!["codex: unreadable".into()];
        let _ = app.view();
        app.error_message = Some("boom".into());
        let _ = app.view();
    }

    #[test]
    fn every_metric_theme_and_window_builds() {
        let mut app = loaded();
        for metric in tokenburn_core::Metric::ALL {
            app.chart_metric = metric;
            for theme in 0..tokenburn_core::THEME_COUNT {
                app.set_theme(theme);
                let _ = app.view();
            }
        }
    }

    #[test]
    fn the_loading_animation_advances_without_overflow() {
        let mut app = loaded();
        app.is_loading = true;
        app.tick = u64::MAX;
        let _ = app.update(Message::AnimationTick);
        assert_eq!(app.tick, 0);
        let _ = app.view();
    }

    #[test]
    fn messages_drive_the_state() {
        let mut app = loaded();
        let _ = app.update(Message::ViewSelected(View::Projects));
        assert_eq!(app.view, View::Projects);
        let _ = app.update(Message::MetricSelected(tokenburn_core::Metric::Cost));
        assert_eq!(app.chart_metric, tokenburn_core::Metric::Cost);
        let _ = app.update(Message::ToolSelected(tokenburn_core::ToolFilter::PI));
        assert_eq!(app.query.tool, tokenburn_core::ToolFilter::PI);
        assert!(app.is_loading);
        let _ = app.update(Message::WindowSelected(tokenburn_core::Window::All));
        assert_eq!(app.query.window, tokenburn_core::Window::All);
        let _ = app.update(Message::Loaded(app.query, Err("disk gone".into())));
        assert_eq!(app.error_message.as_deref(), Some("disk gone"));
        assert!(!app.is_loading);
    }

    #[test]
    fn choosing_a_theme_does_not_write_settings_when_persistence_is_off() {
        let mut app = loaded();
        assert!(!app.persist);
        app.set_theme(usize::MAX); // clamped, never panics
        assert_eq!(app.theme_index, tokenburn_core::THEME_COUNT - 1);
    }

    #[test]
    fn clicking_a_bar_scopes_the_dashboard_and_clicking_again_clears_it() {
        let mut app = loaded();
        let all = app.total();
        let label = app.series.labels[1].clone();
        let _ = app.update(Message::BucketClicked(label.clone()));
        assert_eq!(app.selection.bucket.as_deref(), Some(label.as_str()));
        assert!(app.total().total_tokens < all.total_tokens);
        assert!(app
            .report
            .iter()
            .all(|r| r.bucket.as_deref() == Some(label.as_str()) || r.bucket.is_none()));
        let _ = app.view(); // the "Showing …" chips build
        let _ = app.update(Message::BucketClicked(label));
        assert!(app.selection.is_empty());
        assert_eq!(app.total(), all);
    }

    #[test]
    fn clicking_a_slice_scopes_to_that_project_and_clear_resets() {
        let mut app = loaded();
        let key = app.pie.slices[0].key.clone().unwrap();
        let _ = app.update(Message::SliceClicked(key.clone()));
        assert_eq!(app.selection.project.as_deref(), Some(key.as_str()));
        assert!(app.total().turns < 3);
        let _ = app.view();
        let _ = app.update(Message::ClearSelection);
        assert!(app.selection.is_empty());
        assert_eq!(app.total().turns, 3);
    }

    #[test]
    fn changing_the_query_drops_the_selection() {
        let mut app = loaded();
        let label = app.series.labels[0].clone();
        let _ = app.update(Message::BucketClicked(label));
        let _ = app.update(Message::ToolSelected(tokenburn_core::ToolFilter::PI));
        assert!(app.selection.is_empty());
    }

    #[test]
    fn a_selection_that_vanishes_on_refresh_is_dropped() {
        let mut app = loaded();
        let _ = app.update(Message::SliceClicked(
            app.pie.slices[0].key.clone().unwrap(),
        ));
        app.apply(Collected {
            rows: vec![],
            warnings: vec![],
        });
        assert!(app.selection.is_empty());
    }

    #[test]
    fn the_entry_animation_runs_once_per_view_not_on_every_refresh() {
        let mut app = loaded();
        assert!(app.is_animating(), "the first data grows in");
        assert_eq!(app.anim, 0.0);
        app.tick_animation(std::time::Instant::now() + crate::state::ENTRY_ANIMATION * 2);
        assert_eq!(app.anim, 1.0);
        assert!(
            !app.is_animating(),
            "finished animations switch the frame subscription off"
        );

        // Same query, same metric: a refresh keeps the charts still.
        app.apply(Collected {
            rows: app.rows.clone(),
            warnings: vec![],
        });
        assert!(!app.is_animating());

        // A new metric replays it.
        let _ = app.update(Message::MetricSelected(tokenburn_core::Metric::Cost));
        assert!(app.is_animating());
    }

    #[test]
    fn a_selection_animates_a_pop_and_settles() {
        let mut app = loaded();
        app.tick_animation(std::time::Instant::now() + crate::state::ENTRY_ANIMATION * 2);
        let key = app.pie.slices[0].key.clone().unwrap();
        let _ = app.update(Message::SliceClicked(key));
        assert!(app.is_animating());
        assert_eq!(app.sel_anim, 0.0);
        let _ = app.update(Message::Frame(
            std::time::Instant::now() + crate::state::SELECT_ANIMATION * 2,
        ));
        assert_eq!(app.sel_anim, 1.0);
        assert!(!app.is_animating());
    }

    #[test]
    fn easing_is_monotonic_and_bounded() {
        let mut prev = -1.0;
        for i in 0..=20 {
            let v = crate::state::ease_out(i as f32 / 20.0);
            assert!((0.0..=1.0).contains(&v) && v >= prev);
            prev = v;
        }
        assert_eq!(crate::state::ease_out(2.0), 1.0);
        assert_eq!(crate::state::ease_out(-1.0), 0.0);
    }

    #[test]
    fn the_currency_message_converts_costs_and_every_currency_builds_a_view() {
        let mut app = loaded();
        assert_eq!(app.currency.code, "USD");
        let usd_total = app.total().cost_usd;
        let _ = app.update(Message::CurrencySelected(
            crate::features::currency::CurrencyChoice::of("EUR"),
        ));
        assert_eq!(app.currency.code, "EUR");
        assert!(app.currency.is_approximate());
        // The data is still dollars; only the presentation changed.
        assert_eq!(app.total().cost_usd, usd_total);
        assert_eq!(
            app.currency.format(app.total().cost_usd),
            format!("€{:.2}", usd_total * 0.92)
        );

        for index in 0..tokenburn_core::CURRENCIES.len() {
            app.set_currency(index);
            for metric in tokenburn_core::Metric::ALL {
                app.chart_metric = metric;
                let _ = app.view();
            }
            app.view = View::Projects;
            let _ = app.view();
            app.view = View::Report;
        }
    }

    #[test]
    fn the_users_own_rate_wins_and_is_not_approximate() {
        let mut app = loaded();
        app.rates.insert("EUR".into(), 0.5);
        app.set_currency(tokenburn_core::features::currency::index_of("EUR"));
        assert_eq!(app.currency.per_usd, 0.5);
        assert!(!app.currency.is_approximate());
    }
}
