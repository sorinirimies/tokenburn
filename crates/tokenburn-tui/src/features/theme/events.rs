use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// Keys while the theme picker is open. Always consumes the key.
///
/// Moving the cursor applies (previews) the theme immediately and saves it.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => app.cycle_theme(1),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => app.cycle_theme(-1),
        KeyCode::PageDown => app.cycle_theme(10),
        KeyCode::PageUp => app.cycle_theme(-10),
        KeyCode::Home | KeyCode::Char('g') => app.select_picker(0),
        KeyCode::End | KeyCode::Char('G') => {
            app.select_picker(crate::features::theme::PICKER_LEN - 1)
        }
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('T') | KeyCode::Char('q') => {
            app.theme_picker_open = false;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(app: &mut App, c: KeyCode) {
        handle_key(app, KeyEvent::from(c));
    }

    #[test]
    fn navigation_goes_system_then_presets_and_wraps() {
        let mut app = App::new();
        app.theme_picker_open = true;
        press(&mut app, KeyCode::Down);
        assert_eq!((app.follow_system, app.theme_index), (false, 1));
        press(&mut app, KeyCode::Up);
        press(&mut app, KeyCode::Up);
        assert!(app.follow_system, "System is the first row");
        assert_eq!(app.picker_index(), 0);
        press(&mut app, KeyCode::Up);
        assert_eq!(
            (app.follow_system, app.theme_index),
            (false, tokenburn_core::THEME_COUNT - 1),
            "wraps to the last preset"
        );
        assert_eq!(
            app.theme,
            crate::features::theme::UiTheme::by_index(app.theme_index)
        );
        press(&mut app, KeyCode::Home);
        assert!(app.follow_system);
        press(&mut app, KeyCode::End);
        assert_eq!(app.theme_index, tokenburn_core::THEME_COUNT - 1);
        press(&mut app, KeyCode::PageUp);
        press(&mut app, KeyCode::PageDown);
        assert_eq!(app.picker_index(), crate::features::theme::PICKER_LEN - 1);
    }

    #[test]
    fn system_resolves_to_default_or_default_light() {
        let mut app = App::new();
        app.set_theme(9);
        app.select_picker(0);
        assert!(app.follow_system && app.theme_index <= 1);
        assert_eq!(
            app.theme,
            crate::features::theme::UiTheme::by_index(app.theme_index)
        );
        // Picking a preset stops following.
        app.select_picker(10);
        assert!(!app.follow_system && app.theme_index == 9);
    }

    #[test]
    fn refresh_only_runs_while_following_and_is_throttled() {
        let mut app = App::new();
        app.last_system_check -= std::time::Duration::from_secs(10);
        assert!(!app.refresh_system_theme(), "not following: no-op");
        app.set_follow_system();
        app.theme_index = 40; // pretend the OS flipped
        app.last_system_check = std::time::Instant::now();
        assert!(!app.refresh_system_theme(), "throttled");
        app.last_system_check -= std::time::Duration::from_secs(10);
        assert!(app.refresh_system_theme());
        assert!(app.theme_index <= 1);
    }

    #[test]
    fn apply_theme_choice_does_not_persist_and_sets_follow() {
        let mut app = App::new();
        app.apply_theme_choice(tokenburn_core::ThemeChoice::System);
        assert!(app.follow_system);
        app.apply_theme_choice(tokenburn_core::ThemeChoice::Preset(10));
        assert!(!app.follow_system && app.theme_index == 10);
    }

    #[test]
    fn escape_enter_and_q_close_the_picker_without_quitting() {
        for code in [
            KeyCode::Esc,
            KeyCode::Enter,
            KeyCode::Char('q'),
            KeyCode::Char('T'),
        ] {
            let mut app = App::new();
            app.theme_picker_open = true;
            press(&mut app, code);
            assert!(!app.theme_picker_open && !app.should_quit);
        }
    }
}
