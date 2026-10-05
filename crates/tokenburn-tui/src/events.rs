//! Key dispatch.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use tokenburn_core::Bucket;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    // The options popup captures every key while it is open.
    if app.options.open {
        crate::features::options::events::handle_key(app, key);
        return;
    }

    match key.code {
        KeyCode::Char('o') => app.options.open = true,
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('r') | KeyCode::F(5) => app.reload(),
        KeyCode::Char('t') => app.cycle_tool(),
        KeyCode::Char('w') => app.cycle_window(),
        KeyCode::Char('p') => app.toggle_view(),
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => app.cycle_bucket(true),
        KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => app.cycle_bucket(false),
        KeyCode::Char('1') => app.set_bucket(Bucket::None),
        KeyCode::Char('2') => app.set_bucket(Bucket::Hour),
        KeyCode::Char('3') => app.set_bucket(Bucket::Day),
        KeyCode::Char('4') => app.set_bucket(Bucket::Month),
        _ => crate::features::report::events::handle_nav(app, key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    #[test]
    fn o_opens_options_and_swallows_keys_until_closed() {
        let mut app = App::new();
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('o')));
        assert!(app.options.open);
        // `q` closes the popup instead of quitting the app.
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('q')));
        assert!(!app.options.open);
        assert!(!app.should_quit);
    }

    #[test]
    fn q_quits() {
        let mut app = App::new();
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn ctrl_c_quits() {
        let mut app = App::new();
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );
        assert!(app.should_quit);
    }

    #[test]
    fn digit_selects_bucket_and_window() {
        let mut app = App::new();
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('2')));
        assert_eq!(app.query.bucket, Bucket::Hour);
        assert_eq!(app.query.window, tokenburn_core::Window::Today);
    }
}
