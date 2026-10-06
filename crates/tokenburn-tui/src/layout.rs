//! Top-level screen layout.

use ratatui::layout::{Constraint, Layout};
use ratatui::Frame;

use crate::app::{App, View};
use crate::features::{chart, filter, options, pie, projects, report, theme};
use crate::widgets::{header, status_bar};

pub fn render(app: &mut App, frame: &mut Frame) {
    // Paint the whole screen first so the theme background shows everywhere.
    frame.render_widget(
        ratatui::widgets::Block::default().style(
            ratatui::style::Style::default()
                .bg(app.theme.bg)
                .fg(app.theme.text),
        ),
        frame.area(),
    );

    let [head, body, foot] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(5),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    header::render(app, frame, head);

    match app.view {
        View::Report => {
            let show_chart = app.options.show_chart;
            let show_pie = app.options.show_pie;
            if show_chart || show_pie {
                let [table_area, bottom] =
                    Layout::vertical([Constraint::Min(5), Constraint::Length(13)]).areas(body);
                report::view::render(app, frame, table_area);
                match (show_chart, show_pie) {
                    (true, true) => {
                        let [c, p] = Layout::horizontal([
                            Constraint::Percentage(55),
                            Constraint::Percentage(45),
                        ])
                        .areas(bottom);
                        chart::view::render(app, frame, c);
                        pie::view::render(app, frame, p);
                    }
                    (true, false) => chart::view::render(app, frame, bottom),
                    _ => pie::view::render(app, frame, bottom),
                }
            } else {
                report::view::render(app, frame, body);
            }
        }
        View::Projects => projects::view::render(app, frame, body),
    }

    status_bar::render(app, frame, foot);

    if app.options.open {
        options::view::render(app, frame, frame.area());
    }
    if app.tool_filter_open {
        filter::view::render(app, frame, frame.area());
    }
    if app.theme_picker_open {
        theme::view::render(app, frame, frame.area());
    }
    if app.currency_picker_open {
        crate::features::currency::view::render(app, frame, frame.area());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use tokenburn_core::{Row, Tool};

    fn app_with_rows() -> App {
        let mut app = App::new();
        app.all_rows = vec![Row {
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
        app.rebuild();
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
    fn renders_pie_chart_and_bar_chart_panels() {
        let mut app = app_with_rows();
        let out = draw(&mut app);
        assert!(
            out.contains("Share by project"),
            "pie panel missing:\n{out}"
        );
        assert!(out.contains("Total tokens"), "bar chart panel missing");
    }

    #[test]
    fn panels_can_be_switched_off_in_options() {
        let mut app = app_with_rows();
        app.options.show_pie = false;
        app.options.show_chart = false;
        let out = draw(&mut app);
        assert!(!out.contains("Share by project"));
        assert!(!out.contains("Total tokens"));
    }

    #[test]
    fn options_popup_renders_checkboxes_and_sliders() {
        let mut app = app_with_rows();
        app.options.open = true;
        let out = draw(&mut app);
        for needle in [
            "Options",
            "Auto-refresh",
            "Refresh every",
            "Pie slices",
            "☑",
        ] {
            assert!(out.contains(needle), "missing {needle:?} in:\n{out}");
        }
    }

    #[test]
    fn the_theme_background_covers_the_whole_screen() {
        for index in [0usize, 1, 9, 18, 42] {
            let mut app = app_with_rows();
            app.set_theme(index);
            let mut term = Terminal::new(TestBackend::new(120, 30)).unwrap();
            term.draw(|f| render(&mut app, f)).unwrap();
            let bg = app.theme.bg;
            let buf = term.backend().buffer();
            // The top corners are header-border cells: they keep the root background.
            for (x, y) in [(0, 0), (119, 0)] {
                assert_eq!(buf[(x, y)].bg, bg, "theme {index} at ({x},{y})");
            }
        }
    }

    #[test]
    fn every_theme_renders_every_panel_without_panicking() {
        for index in 0..tokenburn_core::THEME_COUNT {
            let mut app = app_with_rows();
            app.set_theme(index);
            app.options.open = index % 2 == 0;
            app.theme_picker_open = index % 3 == 0;
            let out = draw(&mut app);
            assert!(out.contains("TokenBurn"), "theme {index}");
        }
    }

    #[test]
    fn theme_picker_lists_themes_and_marks_the_current_one() {
        let mut app = app_with_rows();
        app.set_theme(9); // Dracula
        app.theme_picker_open = true;
        let out = draw(&mut app);
        assert!(out.contains("Theme"), "{out}");
        assert!(out.contains("Dracula"), "the current theme is visible");
        assert!(out.contains("10/43"), "position indicator");
    }

    #[test]
    fn the_header_names_the_active_theme() {
        let mut app = app_with_rows();
        app.set_theme(10);
        assert!(draw(&mut app).contains("theme:Nord"));
    }

    #[test]
    fn the_filter_shows_in_the_header_and_hides_non_matching_rows() {
        let mut app = app_with_rows();
        app.set_filter_text("zzz-no-match".into());
        let out = draw(&mut app);
        assert!(out.contains("filter:\"zzz-no-match\""), "{out}");
        assert!(out.contains("0/1 rows"));
        assert!(
            out.contains("The filter hides all 1 rows"),
            "dedicated empty state"
        );
        app.clear_filter();
        assert!(!draw(&mut app).contains("filter:"));
    }

    #[test]
    fn the_text_box_replaces_the_status_line_while_editing() {
        let mut app = app_with_rows();
        app.filter_editing = true;
        app.filter.text = "proj".into();
        let out = draw(&mut app);
        assert!(out.contains("/ proj█"), "{out}");
        assert!(out.contains("Enter keep"));
    }

    #[test]
    fn the_tool_popup_lists_every_tool_with_counts_and_checkboxes() {
        let mut app = app_with_rows();
        app.tool_filter_open = true;
        app.filter.toggle_tool(Tool::Zed);
        app.rebuild();
        let out = draw(&mut app);
        for name in [
            "Tools",
            "pi",
            "Zed",
            "Claude Code",
            "Codex CLI",
            "Amp",
            "7/8 shown",
            "☑",
            "☐",
        ] {
            assert!(out.contains(name), "missing {name:?} in:\n{out}");
        }
    }

    #[test]
    fn renders_empty_state() {
        let mut app = App::new();
        assert!(draw(&mut app).contains("No tokenburn results"));
    }

    #[test]
    fn costs_follow_the_currency_everywhere_and_the_header_says_which() {
        let mut app = app_with_rows();
        let usd_screen = draw(&mut app);
        assert!(
            usd_screen.contains("$0.50") && usd_screen.contains("cur:USD"),
            "{usd_screen}"
        );

        app.set_currency(tokenburn_core::features::currency::index_of("EUR"));
        let eur_screen = draw(&mut app);
        assert!(
            eur_screen.contains("€0.46"),
            "0.50 USD at 0.92: {eur_screen}"
        );
        assert!(
            eur_screen.contains("cur:EUR ≈"),
            "built-in rates are marked approximate"
        );
        assert!(!eur_screen.contains("$0.50"));

        // The user's own rate is not marked approximate.
        app.rates.insert("EUR".into(), 0.5);
        app.set_currency(tokenburn_core::features::currency::index_of("EUR"));
        let own = draw(&mut app);
        assert!(own.contains("€0.25") && own.contains("cur:EUR ") && !own.contains("cur:EUR ≈"));

        app.toggle_view();
        assert!(draw(&mut app).contains("€0.25"), "the projects table too");
    }

    #[test]
    fn the_currency_picker_lists_currencies_with_their_rates() {
        let mut app = app_with_rows();
        app.currency_picker_open = true;
        let out = draw(&mut app);
        assert!(
            out.contains("Currency") && out.contains("EUR") && out.contains("Euro"),
            "{out}"
        );
        assert!(out.contains("fixed estimates"));
        assert!(
            out.contains("settings.json"),
            "tells you where exact rates go"
        );
        app.set_currency(tokenburn_core::features::currency::index_of("JPY"));
        assert!(
            draw(&mut app).contains("150 per $"),
            "the rate shown is the one in use"
        );
    }

    #[test]
    fn the_status_bar_hints_the_currency_key() {
        let mut app = app_with_rows();
        let out = draw(&mut app);
        assert!(out.contains("C currency"), "{out}");
    }
}
