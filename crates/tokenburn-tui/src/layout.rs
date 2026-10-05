//! Top-level screen layout.

use ratatui::layout::{Constraint, Layout};
use ratatui::Frame;

use crate::app::{App, View};
use crate::features::{chart, projects, report};
use crate::widgets::{header, status_bar};

pub fn render(app: &mut App, frame: &mut Frame) {
    let [head, body, foot] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(5),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    header::render(app, frame, head);

    match app.view {
        View::Report => {
            let [table_area, chart_area] =
                Layout::vertical([Constraint::Min(5), Constraint::Length(10)]).areas(body);
            report::view::render(app, frame, table_area);
            chart::view::render(app, frame, chart_area);
        }
        View::Projects => projects::view::render(app, frame, body),
    }

    status_bar::render(app, frame, foot);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use tokenburn_core::{build_report, by_project, Query, Row, Tool};

    fn app_with_rows() -> App {
        let mut app = App::new();
        app.rows = vec![Row {
            tool: Tool::Pi,
            project: "proj".into(),
            id: "x".into(),
            ts: "2026-05-01T10:00:00Z".parse().unwrap(),
            input: 10,
            output: 5,
            cache_read: 1,
            cache_write: 1,
            cost: 0.5,
        }];
        app.report = build_report(&app.rows, &Query::default());
        app.projects = by_project(&app.rows);
        app.clamp_selection();
        app
    }

    fn draw(app: &mut App) -> String {
        let mut term = Terminal::new(TestBackend::new(120, 30)).unwrap();
        term.draw(|f| render(app, f)).unwrap();
        term.backend().to_string()
    }

    #[test]
    fn renders_report_and_projects() {
        let mut app = app_with_rows();
        let out = draw(&mut app);
        assert!(out.contains("TokenBurn"));
        assert!(out.contains("TOTAL"));
        app.toggle_view();
        assert!(draw(&mut app).contains("pi:proj"));
    }

    #[test]
    fn renders_empty_state() {
        let mut app = App::new();
        assert!(draw(&mut app).contains("No tokenburn results"));
    }
}
