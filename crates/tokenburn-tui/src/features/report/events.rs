use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// Row navigation shared by the report and projects tables.
pub fn handle_nav(app: &mut App, key: KeyEvent) {
    let n = app.line_count();
    if n == 0 {
        return;
    }
    let cur = app.table_state.selected().unwrap_or(0);
    let next = match key.code {
        KeyCode::Down | KeyCode::Char('j') => (cur + 1).min(n - 1),
        KeyCode::Up | KeyCode::Char('k') => cur.saturating_sub(1),
        KeyCode::PageDown => (cur + 10).min(n - 1),
        KeyCode::PageUp => cur.saturating_sub(10),
        KeyCode::Home | KeyCode::Char('g') => 0,
        KeyCode::End | KeyCode::Char('G') => n - 1,
        _ => return,
    };
    app.table_state.select(Some(next));
}
