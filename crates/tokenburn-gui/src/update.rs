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
            Message::ThemeSelected(theme) => {
                self.theme = theme;
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
