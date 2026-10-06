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
        KeyCode::Home | KeyCode::Char('g') => app.set_theme(0),
        KeyCode::End | KeyCode::Char('G') => app.set_theme(tokenburn_core::THEME_COUNT - 1),
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
    fn navigation_wraps_and_applies_the_theme() {
        let mut app = App::new();
        app.theme_picker_open = true;
        press(&mut app, KeyCode::Down);
        assert_eq!(app.theme_index, 1);
        press(&mut app, KeyCode::Up);
        press(&mut app, KeyCode::Up);
        assert_eq!(
            app.theme_index,
            tokenburn_core::THEME_COUNT - 1,
            "wraps to the last theme"
        );
        assert_eq!(
            app.theme,
            crate::features::theme::UiTheme::by_index(app.theme_index)
        );
        press(&mut app, KeyCode::Home);
        assert_eq!(app.theme_index, 0);
        press(&mut app, KeyCode::End);
        assert_eq!(app.theme_index, tokenburn_core::THEME_COUNT - 1);
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
