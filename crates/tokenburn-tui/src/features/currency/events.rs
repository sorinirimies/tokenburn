use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// Keys while the currency picker is open. Always consumes the key.
///
/// Like the theme picker, moving the cursor applies (previews) the currency
/// immediately and saves it.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => app.cycle_currency(1),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => app.cycle_currency(-1),
        KeyCode::PageDown => app.cycle_currency(8),
        KeyCode::PageUp => app.cycle_currency(-8),
        KeyCode::Home | KeyCode::Char('g') => app.set_currency(0),
        KeyCode::End | KeyCode::Char('G') => app.set_currency(tokenburn_core::CURRENCIES.len() - 1),
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('C') | KeyCode::Char('q') => {
            app.currency_picker_open = false;
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
    fn navigation_wraps_and_applies_the_currency() {
        let mut app = App::new();
        app.currency_picker_open = true;
        assert_eq!(app.currency.code, "USD");
        press(&mut app, KeyCode::Down);
        assert_eq!(app.currency.code, tokenburn_core::CURRENCIES[1].code);
        press(&mut app, KeyCode::Up);
        press(&mut app, KeyCode::Up);
        assert_eq!(
            app.currency.code,
            tokenburn_core::CURRENCIES.last().unwrap().code,
            "wraps to the last currency"
        );
        press(&mut app, KeyCode::Home);
        assert_eq!(app.currency.code, "USD");
        press(&mut app, KeyCode::End);
        assert_eq!(app.currency.code, "ZAR");
    }

    #[test]
    fn escape_enter_q_and_c_close_the_picker_without_quitting() {
        for code in [
            KeyCode::Esc,
            KeyCode::Enter,
            KeyCode::Char('q'),
            KeyCode::Char('C'),
        ] {
            let mut app = App::new();
            app.currency_picker_open = true;
            press(&mut app, code);
            assert!(!app.currency_picker_open && !app.should_quit);
        }
    }

    #[test]
    fn other_keys_are_swallowed() {
        let mut app = App::new();
        app.currency_picker_open = true;
        press(&mut app, KeyCode::Char('t'));
        assert!(
            app.currency_picker_open,
            "`t` must not cycle the tool underneath"
        );
    }
}
