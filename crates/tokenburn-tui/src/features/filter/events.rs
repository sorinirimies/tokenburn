use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokenburn_core::Tool;

use crate::app::App;

/// Keys while the `/` text box is active. Always consumes the key.
///
/// Typing filters live. `Enter` keeps the filter and returns to browsing;
/// `Esc` clears it.
pub fn handle_text_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => app.filter_editing = false,
        KeyCode::Esc => {
            app.filter_editing = false;
            app.set_filter_text(String::new());
        }
        KeyCode::Backspace => {
            let mut t = app.filter.text.clone();
            t.pop();
            app.set_filter_text(t);
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.set_filter_text(String::new());
        }
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Delete the last word.
            let t = app.filter.text.trim_end();
            let cut = t.rfind(' ').map_or(0, |i| i + 1);
            let t = t[..cut].to_string();
            app.set_filter_text(t);
        }
        KeyCode::Char(c)
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT) =>
        {
            let mut t = app.filter.text.clone();
            t.push(c);
            app.set_filter_text(t);
        }
        _ => {}
    }
}

/// Keys while the tool-checkbox popup is open. Always consumes the key.
pub fn handle_tool_key(app: &mut App, key: KeyEvent) {
    let n = Tool::ALL.len();
    match key.code {
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
            app.tool_cursor = (app.tool_cursor + 1) % n
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => {
            app.tool_cursor = (app.tool_cursor + n - 1) % n;
        }
        KeyCode::Char(' ') | KeyCode::Enter | KeyCode::Right | KeyCode::Left => {
            app.toggle_tool_at_cursor();
        }
        KeyCode::Char('a') => {
            app.filter.select_all_tools();
            app.rebuild();
        }
        KeyCode::Char('n') => {
            app.filter.select_no_tools();
            app.rebuild();
        }
        KeyCode::Char('x') => app.clear_filter(),
        KeyCode::Esc | KeyCode::Char('f') | KeyCode::Char('q') => app.tool_filter_open = false,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;
    use tokenburn_core::Row;

    fn row(tool: Tool, project: &str) -> Row {
        Row {
            tool,
            project: project.into(),
            id: "s".into(),
            ts: "2026-05-01T12:00:00Z".parse().unwrap(),
            input: 10,
            output: 5,
            cache_read: 0,
            cache_write: 0,
            cost: 1.0,
        }
    }

    fn app() -> App {
        let mut a = App::new();
        a.all_rows = vec![
            row(Tool::Pi, "alpha"),
            row(Tool::Pi, "beta"),
            row(Tool::Claude, "alpha"),
        ];
        a.rebuild();
        a
    }

    fn type_text(a: &mut App, s: &str) {
        for c in s.chars() {
            handle_text_key(a, KeyEvent::from(KeyCode::Char(c)));
        }
    }

    #[test]
    fn typing_filters_rows_live_and_updates_the_totals() {
        let mut a = app();
        assert_eq!(a.rows_len, 3);
        a.filter_editing = true;
        type_text(&mut a, "alp");
        assert_eq!(a.rows_len, 2);
        assert_eq!(a.total().turns, 2, "the header totals follow the filter");
        assert_eq!(a.projects.len(), 2, "projects / pie follow the filter");
        type_text(&mut a, "ha -claude");
        assert_eq!(a.rows_len, 1);
    }

    #[test]
    fn backspace_ctrl_u_and_ctrl_w_edit_the_text() {
        let mut a = app();
        type_text(&mut a, "alpha beta");
        handle_text_key(&mut a, KeyEvent::from(KeyCode::Backspace));
        assert_eq!(a.filter.text, "alpha bet");
        handle_text_key(
            &mut a,
            KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
        );
        assert_eq!(a.filter.text, "alpha ");
        handle_text_key(
            &mut a,
            KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        );
        assert_eq!(a.filter.text, "");
        assert_eq!(a.rows_len, 3);
    }

    #[test]
    fn enter_keeps_the_filter_and_escape_clears_it() {
        let mut a = app();
        a.filter_editing = true;
        type_text(&mut a, "beta");
        handle_text_key(&mut a, KeyEvent::from(KeyCode::Enter));
        assert!(!a.filter_editing);
        assert_eq!(a.rows_len, 1, "Enter keeps it");

        a.filter_editing = true;
        handle_text_key(&mut a, KeyEvent::from(KeyCode::Esc));
        assert!(!a.filter_editing);
        assert_eq!(a.rows_len, 3, "Esc clears it");
    }

    #[test]
    fn control_chords_are_not_typed_into_the_box() {
        let mut a = app();
        handle_text_key(
            &mut a,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );
        assert_eq!(a.filter.text, "");
    }

    #[test]
    fn tool_popup_toggles_tools_and_filters_rows() {
        let mut a = app();
        a.tool_filter_open = true;
        // cursor 0 = pi (Tool::ALL[0])
        handle_tool_key(&mut a, KeyEvent::from(KeyCode::Char(' ')));
        assert!(!a.filter.allows(Tool::Pi));
        assert_eq!(a.rows_len, 1, "only the Claude row remains");
        handle_tool_key(&mut a, KeyEvent::from(KeyCode::Char(' ')));
        assert!(
            a.filter.is_empty(),
            "re-enabling every tool clears the filter"
        );
    }

    #[test]
    fn tool_popup_all_none_and_clear() {
        let mut a = app();
        handle_tool_key(&mut a, KeyEvent::from(KeyCode::Char('n')));
        assert_eq!(a.rows_len, 0);
        handle_tool_key(&mut a, KeyEvent::from(KeyCode::Char('a')));
        assert_eq!(a.rows_len, 3);
        type_text(&mut a, "beta");
        handle_tool_key(&mut a, KeyEvent::from(KeyCode::Char('x')));
        assert_eq!(a.rows_len, 3, "x clears text and tool filters");
    }

    #[test]
    fn tool_popup_cursor_wraps_and_esc_closes() {
        let mut a = app();
        a.tool_filter_open = true;
        handle_tool_key(&mut a, KeyEvent::from(KeyCode::Up));
        assert_eq!(a.tool_cursor, Tool::ALL.len() - 1);
        handle_tool_key(&mut a, KeyEvent::from(KeyCode::Down));
        assert_eq!(a.tool_cursor, 0);
        handle_tool_key(&mut a, KeyEvent::from(KeyCode::Esc));
        assert!(!a.tool_filter_open);
    }

    #[test]
    fn counts_per_tool_come_from_all_rows_not_the_filtered_ones() {
        let mut a = app();
        type_text(&mut a, "beta");
        assert_eq!(a.tool_counts[Tool::Pi.series_index()], 2);
        assert_eq!(a.tool_counts[Tool::Claude.series_index()], 1);
    }
}
