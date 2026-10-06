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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use tokenburn_core::{Row, Tool};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn app_with(n: usize) -> App {
        let mut app = App::new();
        app.all_rows = (0..n)
            .map(|i| Row {
                tool: Tool::Pi,
                project: format!("p{i}"),
                id: "x".into(),
                ts: (chrono::DateTime::parse_from_rfc3339("2026-01-01T10:00:00Z").unwrap()
                    + chrono::Duration::days(i as i64))
                .with_timezone(&chrono::Utc),
                input: 1,
                output: 1,
                cache_read: 0,
                cache_write: 0,
                cost: 0.0,
            })
            .collect();
        app.query.bucket = tokenburn_core::Bucket::Day;
        app.query.window = tokenburn_core::Window::All;
        app.rebuild();
        app.table_state.select(Some(0));
        app
    }

    fn sel(app: &App) -> usize {
        app.table_state.selected().unwrap()
    }

    #[test]
    fn arrows_and_vim_keys_move_and_clamp_at_both_ends() {
        let mut app = app_with(5);
        let n = app.line_count();
        assert!(n >= 5, "one report line per day");
        handle_nav(&mut app, key(KeyCode::Up));
        assert_eq!(sel(&app), 0, "clamped at the top");
        handle_nav(&mut app, key(KeyCode::Down));
        assert_eq!(sel(&app), 1);
        for _ in 0..n + 3 {
            handle_nav(&mut app, key(KeyCode::Char('j')));
        }
        assert_eq!(sel(&app), n - 1, "clamped at the bottom");
        handle_nav(&mut app, key(KeyCode::Char('k')));
        assert_eq!(sel(&app), n - 2);
    }

    #[test]
    fn paging_and_home_end_jump() {
        let mut app = app_with(40);
        let n = app.line_count();
        handle_nav(&mut app, key(KeyCode::PageDown));
        assert_eq!(sel(&app), 10);
        handle_nav(&mut app, key(KeyCode::End));
        assert_eq!(sel(&app), n - 1);
        handle_nav(&mut app, key(KeyCode::PageUp));
        assert_eq!(sel(&app), n - 11);
        handle_nav(&mut app, key(KeyCode::Char('g')));
        assert_eq!(sel(&app), 0);
        handle_nav(&mut app, key(KeyCode::Char('G')));
        assert_eq!(sel(&app), n - 1);
        handle_nav(&mut app, key(KeyCode::Home));
        assert_eq!(sel(&app), 0);
    }

    #[test]
    fn unrelated_keys_and_empty_tables_change_nothing() {
        let mut app = app_with(3);
        handle_nav(&mut app, key(KeyCode::Char('z')));
        assert_eq!(sel(&app), 0);

        let mut empty = App::new();
        empty.table_state.select(None);
        handle_nav(&mut empty, key(KeyCode::Down));
        assert_eq!(empty.table_state.selected(), None);
    }
}
