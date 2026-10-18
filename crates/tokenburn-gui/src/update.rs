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
        Task::batch([commands::load(self.query), self.check_limits()])
    }

    fn check_limits(&self) -> Task<Message> {
        commands::check_limits(self.persist, self.currency)
    }

    /// Like [`Self::reload`] but keeps the status line (the periodic refresh must not
    /// flash "Loading…" every few seconds).
    fn reload_quietly(&mut self) -> Task<Message> {
        self.is_loading = true;
        Task::batch([commands::load(self.query), self.check_limits()])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // The periodic tick never piles up loads: skip it while one is running.
            Message::AutoRefresh if self.is_loading => Task::none(),
            Message::AutoRefresh => self.reload_quietly(),
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
                // The window exists by the time the first data arrives: now is when
                // the platform-specific setup can run (once).
                if self.platform_ready {
                    Task::none()
                } else {
                    self.platform_ready = true;
                    crate::window::setup()
                }
            }
            Message::ToolSelected(tool) => {
                self.query.tool = tool;
                self.selection.clear();
                self.reload()
            }
            Message::BucketSelected(bucket) => {
                self.query.bucket = bucket;
                self.query.window = bucket.default_window();
                self.selection.clear();
                self.reload()
            }
            Message::WindowSelected(window) => {
                self.query.window = window;
                self.selection.clear();
                self.reload()
            }
            Message::ThemeSelected(choice) => {
                self.select_theme(choice);
                Task::none()
            }
            Message::SystemThemeTick => {
                self.refresh_system_theme();
                Task::none()
            }
            Message::CurrencySelected(choice) => {
                self.set_currency(choice.index);
                Task::none()
            }
            Message::MetricSelected(metric) => {
                self.chart_metric = metric;
                self.restart_entry_animation();
                Task::none()
            }
            Message::BucketClicked(label) => {
                self.click_bucket(&label);
                Task::none()
            }
            Message::SliceClicked(key) => {
                self.click_project(&key);
                Task::none()
            }
            Message::ClearSelection => {
                self.clear_selection();
                Task::none()
            }
            Message::Frame(now) => {
                self.tick_animation(now);
                Task::none()
            }
            Message::ViewSelected(view) => {
                self.view = view;
                Task::none()
            }
            Message::LimitsChecked(check) => {
                self.apply_limit_check(check);
                Task::none()
            }
            Message::LimitInput(period, text) => {
                let i = tokenburn_core::Period::ALL
                    .iter()
                    .position(|p| *p == period)
                    .unwrap_or(0);
                self.limit_inputs[i] = text;
                Task::none()
            }
            Message::LimitsApplied => {
                if self.apply_limit_inputs() {
                    self.error_message = None;
                    self.limit_statuses.clear();
                    self.check_limits()
                } else {
                    self.error_message = Some("Limits must be numbers (US dollars).".into());
                    Task::none()
                }
            }
            Message::LimitNotifyToggled(on) => {
                self.limits.notify = on;
                if self.persist {
                    let _ = tokenburn_core::settings::save_limits(&self.limits);
                }
                Task::none()
            }
            Message::DismissAlert => {
                self.alert = None;
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
mod auto_refresh_tests {
    use super::*;
    use tokenburn_core::Collected;

    #[test]
    fn the_periodic_tick_starts_a_load_and_keeps_the_status_line() {
        let mut app = TokenBurn::new();
        app.status_message = Some("Updated 10:00:00".into());
        let _ = app.update(Message::AutoRefresh);
        assert!(app.is_loading);
        assert_eq!(app.status_message.as_deref(), Some("Updated 10:00:00"));
    }

    #[test]
    fn ticks_never_pile_up_while_a_load_is_in_flight() {
        let mut app = TokenBurn::new();
        app.is_loading = true;
        let _ = app.update(Message::AutoRefresh);
        assert!(app.is_loading);
    }

    #[test]
    fn a_finished_load_stamps_the_update_time() {
        let mut app = TokenBurn::new();
        let q = app.query;
        assert!(app.last_updated.is_none());
        let _ = app.update(Message::Loaded(q, Ok(Collected::default())));
        assert!(app.last_updated.is_some());
        let status = app.status_message.unwrap();
        assert!(
            status.starts_with("Updated ") && status.contains("auto-refresh every 10 s"),
            "{status}"
        );
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
    fn selecting_system_follows_and_a_preset_stops_following() {
        let mut app = TokenBurn::new();
        let _ = app.update(Message::ThemeSelected(ThemeChoice::system()));
        assert!(app.follow_system);
        assert!(
            app.theme_index <= 1,
            "System maps to Default / Default Light"
        );
        let _ = app.update(Message::ThemeSelected(ThemeChoice::at(9)));
        assert!(!app.follow_system);
        assert_eq!(app.theme_index, 9);
    }

    #[test]
    fn the_system_tick_only_changes_the_theme_while_following() {
        let mut app = TokenBurn::new();
        app.theme_index = 9;
        let _ = app.update(Message::SystemThemeTick);
        assert_eq!(app.theme_index, 9, "not following: untouched");
        app.follow_system = true;
        let _ = app.update(Message::SystemThemeTick);
        assert_eq!(
            app.theme_index,
            tokenburn_core::current_system_theme_index()
        );
    }

    #[test]
    fn apply_theme_choice_sets_follow_without_persisting() {
        let mut app = TokenBurn::new();
        app.apply_theme_choice(tokenburn_core::ThemeChoice::System);
        assert!(app.follow_system);
        app.apply_theme_choice(tokenburn_core::ThemeChoice::Preset(7));
        assert!(!app.follow_system && app.theme_index == 7);
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

    #[test]
    fn platform_setup_is_requested_once_after_the_first_data() {
        let mut app = TokenBurn::new();
        assert!(!app.platform_ready);
        let q = app.query;
        let _ = app.update(Message::Loaded(q, Ok(tokenburn_core::Collected::default())));
        assert!(
            app.platform_ready,
            "the first load triggers the one-time setup"
        );
        // Later refreshes must not ask again (the menu would be added twice).
        app.platform_ready = true;
        let _ = app.update(Message::Loaded(q, Ok(tokenburn_core::Collected::default())));
        assert!(app.platform_ready);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_window_menu_refuses_to_run_off_the_main_thread_instead_of_crashing() {
        // Test threads are never the main thread, and AppKit must only be touched there.
        assert!(!crate::window::macos::install_window_menu());
    }

    #[test]
    fn limit_fields_parse_apply_and_reject_bad_input() {
        use tokenburn_core::Period;
        let mut app = TokenBurn::new();
        let _ = app.update(Message::LimitInput(Period::Day, "20".into()));
        let _ = app.update(Message::LimitInput(Period::Week, " 100.5 ".into()));
        let _ = app.update(Message::LimitInput(Period::Month, "".into()));
        let _ = app.update(Message::LimitsApplied);
        assert_eq!(app.limits.get(Period::Day), Some(20.0));
        assert_eq!(app.limits.get(Period::Week), Some(100.5));
        assert_eq!(app.limits.get(Period::Month), None);
        assert!(app.error_message.is_none());
        // `0` clears.
        let _ = app.update(Message::LimitInput(Period::Day, "0".into()));
        let _ = app.update(Message::LimitsApplied);
        assert_eq!(app.limits.get(Period::Day), None);
        // Not a number (or negative): keep the old limits and say so.
        for bad in ["abc", "-5", "inf"] {
            let _ = app.update(Message::LimitInput(Period::Week, bad.into()));
            let _ = app.update(Message::LimitsApplied);
            assert_eq!(app.limits.get(Period::Week), Some(100.5), "{bad}");
            assert!(app.error_message.as_deref().unwrap().contains("numbers"));
        }
    }

    #[test]
    fn a_check_updates_the_bars_and_the_fields_and_the_banner_dismisses() {
        use tokenburn_core::{Level, LimitCheck, LimitStatus, Limits, Period};
        let mut app = TokenBurn::new();
        let mut limits = Limits::default();
        limits.set(Period::Day, Some(20.0));
        let _ = app.update(Message::LimitsChecked(LimitCheck {
            limits: limits.clone(),
            statuses: vec![LimitStatus {
                period: Period::Day,
                limit_usd: 20.0,
                spent_usd: 25.0,
                level: Level::Over,
            }],
            alerts: vec![tokenburn_core::Alert {
                period: Period::Day,
                level: Level::Over,
                spent_usd: 25.0,
                limit_usd: 20.0,
            }],
        }));
        assert_eq!(app.limit_statuses.len(), 1);
        assert_eq!(
            app.limit_inputs[0], "20",
            "the Settings field follows the saved limit"
        );
        assert!(app
            .alert
            .as_deref()
            .unwrap()
            .contains("daily limit exceeded"));
        // The page and the Settings panel build with all of it.
        for view in [crate::state::View::Report, crate::state::View::Settings] {
            app.view = view;
            for l in [
                tokenburn_core::LayoutClass::Wide,
                tokenburn_core::LayoutClass::Narrow,
            ] {
                let _ = app.view_for(l);
            }
        }
        let _ = app.update(Message::DismissAlert);
        assert!(app.alert.is_none());
        let _ = app.update(Message::LimitNotifyToggled(false));
        assert!(!app.limits.notify);
    }
}
