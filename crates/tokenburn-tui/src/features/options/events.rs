use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// Keys while the options popup is open. Always consumes the key.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    let o = &mut app.options;
    match key.code {
        KeyCode::Esc | KeyCode::Char('o') | KeyCode::Char('q') => o.open = false,
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => o.move_cursor(1),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => o.move_cursor(-1),
        KeyCode::Char(' ') | KeyCode::Enter => o.toggle(),
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('+') => o.adjust(1),
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('-') => o.adjust(-1),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::options::OptionRow;

    fn press(app: &mut App, c: KeyCode) {
        handle_key(app, KeyEvent::from(c));
    }

    #[test]
    fn navigation_toggle_and_close() {
        let mut app = App::new();
        app.options.open = true;
        press(&mut app, KeyCode::Down);
        assert_eq!(app.options.cursor, OptionRow::RefreshSecs);
        press(&mut app, KeyCode::Right);
        assert_eq!(app.options.refresh_interval().as_secs(), 10);
        press(&mut app, KeyCode::Up);
        press(&mut app, KeyCode::Char(' '));
        assert!(!app.options.auto_refresh);
        press(&mut app, KeyCode::Esc);
        assert!(!app.options.open);
    }
}
