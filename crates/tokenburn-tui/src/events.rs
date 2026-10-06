//! Key dispatch.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use tokenburn_core::Bucket;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    // Popups capture every key while they are open.
    if app.theme_picker_open {
        crate::features::theme::events::handle_key(app, key);
        return;
    }
    if app.options.open {
        crate::features::options::events::handle_key(app, key);
        return;
    }

    match key.code {
        KeyCode::Char('o') => app.options.open = true,
        KeyCode::Char('T') => app.theme_picker_open = true,
        KeyCode::Char(']') => app.cycle_theme(1),
        KeyCode::Char('[') => app.cycle_theme(-1),
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
    fn capital_t_opens_the_theme_picker_and_brackets_cycle_themes() {
        let mut app = App::new();
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('T')));
        assert!(app.theme_picker_open);
        handle_key(&mut app, KeyEvent::from(KeyCode::Esc));
        assert!(!app.theme_picker_open);
        handle_key(&mut app, KeyEvent::from(KeyCode::Char(']')));
        assert_eq!(app.theme_index, 1);
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('[')));
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('[')));
        assert_eq!(app.theme_index, tokenburn_core::THEME_COUNT - 1);
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
