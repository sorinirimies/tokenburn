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
}
