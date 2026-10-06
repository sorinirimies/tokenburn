//! The Elm `update` function.

use iced::Task;

use crate::features::report::commands;
use crate::message::Message;
use crate::state::TokenBurn;

impl TokenBurn {
    /// Start a background reload for the current query.
    pub fn reload(&mut self) -> Task<Message> {
        self.is_loading = true;
        self.status_message = Some("Loading…".into());
        commands::load(self.query)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Refresh => self.reload(),
            Message::Loaded(query, result) => {
                // Drop results for a query the user has already changed.
                if query != self.query {
                    return Task::none();
                }
                self.is_loading = false;
                match result {
                    Ok(collected) => self.apply(collected),
                    Err(e) => self.error_message = Some(e),
                }
                Task::none()
            }
            Message::ToolSelected(tool) => {
                self.query.tool = tool;
                self.reload()
            }
            Message::BucketSelected(bucket) => {
                self.query.bucket = bucket;
                self.query.window = bucket.default_window();
                self.reload()
            }
            Message::WindowSelected(window) => {
                self.query.window = window;
                self.reload()
            }
            Message::ThemeSelected(choice) => {
                self.set_theme(choice.index);
                Task::none()
            }
            Message::ViewSelected(view) => {
                self.view = view;
                Task::none()
            }
            Message::AnimationTick => {
                self.tick = self.tick.wrapping_add(1);
                Task::none()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokenburn_core::{Bucket, Collected};

    #[test]
    fn bucket_selection_resets_window() {
        let mut app = TokenBurn::new();
        let _ = app.update(Message::BucketSelected(Bucket::Hour));
        assert_eq!(app.query.window, tokenburn_core::Window::Today);
        assert!(app.is_loading);
    }

    #[test]
    fn stale_results_are_ignored() {
        let mut app = TokenBurn::new();
        let old = app.query;
        let _ = app.update(Message::BucketSelected(Bucket::Day));
        let _ = app.update(Message::Loaded(old, Ok(Collected::default())));
        assert!(app.is_loading, "stale result must not clear loading");
    }

    #[test]
    fn loaded_clears_loading() {
        let mut app = TokenBurn::new();
        let q = app.query;
        app.is_loading = true;
        let _ = app.update(Message::Loaded(q, Ok(Collected::default())));
        assert!(!app.is_loading);
    }
}

#[cfg(test)]
mod theme_tests {
    use super::*;
    use crate::features::theme::ThemeChoice;

    #[test]
    fn selecting_a_theme_changes_the_iced_theme() {
        let mut app = TokenBurn::new();
        let before = app.iced_theme();
        let _ = app.update(Message::ThemeSelected(ThemeChoice::at(9)));
        assert_eq!(app.theme_index, 9);
        assert_ne!(
            app.iced_theme().palette().background,
            before.palette().background
        );
        assert_eq!(app.iced_theme().to_string(), tokenburn_core::THEME_NAMES[9]);
    }

    #[test]
    fn persistence_is_off_by_default_so_tests_never_touch_the_users_settings() {
        let mut app = TokenBurn::new();
        assert!(!app.persist);
        app.set_theme(5); // must not write anything
        assert_eq!(app.theme_index, 5);
        app.set_theme(usize::MAX);
        assert_eq!(app.theme_index, tokenburn_core::THEME_COUNT - 1, "clamped");
    }
}
