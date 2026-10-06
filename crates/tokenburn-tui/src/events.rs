//! Key dispatch.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use tokenburn_core::Bucket;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    // The `/` text box and the popups capture every key while they are active.
    if app.filter_editing {
        crate::features::filter::events::handle_text_key(app, key);
        return;
    }
    if app.tool_filter_open {
        crate::features::filter::events::handle_tool_key(app, key);
        return;
    }
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
        KeyCode::Char('/') => app.filter_editing = true,
        KeyCode::Char('f') => app.tool_filter_open = true,
        // Esc first clears an active filter; only a second Esc quits.
        KeyCode::Esc if !app.filter.is_empty() => app.clear_filter(),
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
    fn slash_starts_the_text_filter_and_f_opens_the_tool_popup() {
        let mut app = App::new();
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('/')));
        assert!(app.filter_editing);
        // While editing, letters are text — `q` must not quit.
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('q')));
        assert!(!app.should_quit);
        assert_eq!(app.filter.text, "q");
        handle_key(&mut app, KeyEvent::from(KeyCode::Enter));
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('f')));
        assert!(app.tool_filter_open);
    }

    #[test]
    fn escape_clears_an_active_filter_before_it_quits() {
        let mut app = App::new();
        app.set_filter_text("x".into());
        handle_key(&mut app, KeyEvent::from(KeyCode::Esc));
        assert!(
            app.filter.is_empty() && !app.should_quit,
            "first Esc clears the filter"
        );
        handle_key(&mut app, KeyEvent::from(KeyCode::Esc));
        assert!(app.should_quit, "second Esc quits");
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
