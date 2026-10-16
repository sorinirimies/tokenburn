//! Top-level screen layout.

use ratatui::layout::{Constraint, Layout};
use ratatui::Frame;

use crate::app::{App, View};
use crate::features::{chart, filter, options, pie, projects, report, theme};
use crate::widgets::{header, status_bar};
use tokenburn_core::LayoutClass;

/// Smallest terminal the UI draws in; below it a "too small" notice is shown.
pub const MIN_WIDTH: u16 = 40;
pub const MIN_HEIGHT: u16 = 10;

/// Rows given to the charts under the table on a terminal `height` rows tall: the
/// full 13 when there is room, a compact 9, none on short terminals (the table
/// matters more than the picture).
pub fn chart_height(height: u16) -> u16 {
    match height {
        28.. => 13,
        22..=27 => 9,
        _ => 0,
    }
}

fn render_too_small(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let msg = format!(
        "TokenBurn needs at least {MIN_WIDTH}×{MIN_HEIGHT}\nnow {}×{}",
        area.width, area.height
    );
    frame.render_widget(
        ratatui::widgets::Paragraph::new(msg)
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true })
            .style(ratatui::style::Style::default().fg(app.theme.warning)),
        area,
    );
}

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

    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        render_too_small(app, frame, area);
        return;
    }
    let class = LayoutClass::from_cols(area.width);

    let [head, body, foot] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(5),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    header::render(app, frame, head);

    match app.view {
        View::Report => {
            // What fits: the charts need height (they vanish on short terminals and
            // shrink on small ones) and the pie needs width (it vanishes when narrow).
            let chart_rows = chart_height(area.height);
            let show_chart = app.options.show_chart && chart_rows > 0;
            let show_pie = app.options.show_pie && chart_rows > 0 && !class.is_narrow();
            if show_chart || show_pie {
                let [table_area, bottom] =
                    Layout::vertical([Constraint::Min(5), Constraint::Length(chart_rows)])
                        .areas(body);
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
            "Settings",
            "About",
            "Auto-refresh",
            "Refresh every",
            "Pie slices",
            "☑",
        ] {
            assert!(out.contains(needle), "missing {needle:?} in:\n{out}");
        }
    }

    #[test]
    fn title_shows_the_version_and_about_tab_renders() {
        let mut app = app_with_rows();
        let v = tokenburn_core::appinfo::version_label();
        assert!(draw(&mut app).contains(&v));
        app.options.open_tab(crate::features::options::Tab::About);
        let out = draw(&mut app);
        for needle in ["Developer", "GitHub", "License"] {
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
        assert!(out.contains("11/44"), "position indicator");
    }

    #[test]
    fn theme_picker_lists_system_first_and_marks_it_when_following() {
        let mut app = app_with_rows();
        app.set_follow_system();
        app.theme_picker_open = true;
        let out = draw(&mut app);
        assert!(out.contains("System"), "{out}");
        assert!(out.contains("1/44"), "System is row 1: {out}");
        app.theme_picker_open = false;
        assert!(draw(&mut app).contains("theme:System"));
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
            "11/12 shown",
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

    fn draw_at(app: &mut App, w: u16, h: u16) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render(app, f)).unwrap();
        term.backend().to_string()
    }

    fn day_app() -> App {
        let mut app = app_with_rows();
        app.query.bucket = tokenburn_core::Bucket::Day;
        app.query.window = tokenburn_core::Window::All;
        app.rebuild();
        app
    }

    #[test]
    fn wide_terminals_show_every_column_and_both_charts() {
        let mut app = day_app();
        let out = draw_at(&mut app, 160, 40);
        for col in [
            "turns",
            "input",
            "output",
            "cache_read",
            "cache_write",
            "total",
            "cost",
        ] {
            assert!(out.contains(col), "{col} missing:\n{out}");
        }
        assert!(
            out.contains("cache r/w"),
            "the header shows the cache totals"
        );
        assert!(out.contains("theme:") && out.contains("cur:"));
        assert!(out.contains("Share by project") && out.contains("Total tokens"));
    }

    #[test]
    fn medium_terminals_drop_the_cache_columns_but_keep_the_charts() {
        let mut app = day_app();
        let out = draw_at(&mut app, 90, 30);
        assert!(out.contains("input") && out.contains("output") && out.contains("total"));
        assert!(
            !out.contains("cache_read") && !out.contains("cache_write"),
            "{out}"
        );
        assert!(
            !out.contains("cache r/w"),
            "no cache totals in the header either"
        );
        assert!(out.contains("Share by project") && out.contains("Total tokens"));
    }

    #[test]
    fn narrow_terminals_keep_only_the_essentials() {
        let mut app = day_app();
        let out = draw_at(&mut app, 60, 30);
        assert!(out.contains("total") && out.contains("cost") && out.contains("tool"));
        // (The header keeps "turns" in its totals; look at the table's own header row.)
        let table_head = out.lines().find(|l| l.contains("bucket")).unwrap();
        for gone in ["turns", "input", "output", "cache_"] {
            assert!(
                !table_head.contains(gone),
                "{gone} should be hidden when narrow:\n{out}"
            );
        }
        assert!(!out.contains("input") && !out.contains("cache_"));
        assert!(!out.contains("theme:"), "the theme chip makes room");
        assert!(
            out.contains("cur:USD"),
            "the currency stays: it says what the cost means"
        );
        assert!(!out.contains("Share by project"), "no pie when narrow");
        assert!(out.contains("Total tokens"), "the bar chart stays");
        assert!(out.contains("C cur"), "short hints");
        assert!(
            !out.contains("auto-refresh"),
            "no room for the status message"
        );
    }

    #[test]
    fn the_classic_80_by_24_is_usable() {
        let mut app = day_app();
        let out = draw_at(&mut app, 80, 24);
        assert!(out.contains("TokenBurn") && out.contains("total") && out.contains("cost"));
        assert!(out.contains("2026-05-01"), "the table has rows:\n{out}");
    }

    #[test]
    fn short_terminals_trade_the_charts_for_table_rows() {
        let mut app = day_app();
        assert_eq!(chart_height(40), 13);
        assert_eq!(chart_height(28), 13);
        assert_eq!(chart_height(24), 9);
        assert_eq!(chart_height(21), 0);
        let out = draw_at(&mut app, 120, 18);
        assert!(!out.contains("Total tokens") && !out.contains("Share by project"));
        assert!(out.contains("2026-05-01"));
    }

    #[test]
    fn a_terminal_that_is_too_small_says_so_instead_of_garbling() {
        let mut app = day_app();
        let out = draw_at(&mut app, 30, 8);
        assert!(out.contains("needs at least 40"), "{out}");
        assert!(out.contains("30×8"));
        // And it recovers when the window grows again.
        assert!(draw_at(&mut app, 100, 30).contains("TokenBurn"));
    }

    #[test]
    fn no_size_panics_with_or_without_popups_open() {
        let mut app = day_app();
        for popup in 0..6 {
            app.options.open = popup == 1;
            app.theme_picker_open = popup == 2;
            app.currency_picker_open = popup == 3;
            app.tool_filter_open = popup == 4;
            app.filter_editing = popup == 5;
            for (w, h) in [
                (1, 1),
                (10, 5),
                (39, 9),
                (40, 10),
                (41, 11),
                (69, 20),
                (70, 24),
                (79, 24),
                (109, 30),
                (110, 30),
                (200, 60),
                (300, 100),
                (60, 200),
            ] {
                let _ = draw_at(&mut app, w, h);
            }
        }
        // The other view and an empty state, too.
        app.options.open = false;
        app.toggle_view();
        for (w, h) in [(40, 10), (65, 20), (150, 50)] {
            let _ = draw_at(&mut app, w, h);
        }
        let mut empty = App::new();
        for (w, h) in [(40, 10), (65, 20), (150, 50)] {
            let _ = draw_at(&mut empty, w, h);
        }
    }

    #[test]
    fn the_terminal_classes_match_the_shared_breakpoints() {
        assert_eq!(LayoutClass::from_cols(160), LayoutClass::Wide);
        assert_eq!(LayoutClass::from_cols(90), LayoutClass::Medium);
        assert_eq!(LayoutClass::from_cols(60), LayoutClass::Narrow);
        const { assert!(MIN_WIDTH < tokenburn_core::utils::layout::MEDIUM_COLS) };
    }

    #[test]
    fn the_projects_view_drops_the_turns_column_when_narrow() {
        let mut app = day_app();
        app.toggle_view();
        let wide = draw_at(&mut app, 120, 30);
        assert!(wide.contains("turns"));
        let narrow = draw_at(&mut app, 60, 30);
        let head = narrow.lines().find(|l| l.contains("tool:project")).unwrap();
        assert!(
            !head.contains("turns") && head.contains("total") && head.contains("cost"),
            "{narrow}"
        );
    }

    #[test]
    fn the_narrow_header_fits_even_on_a_very_narrow_terminal() {
        let mut app = day_app();
        // ~55 columns: tool, view, window and currency all fit on the title line.
        let out = draw_at(&mut app, 55, 24);
        let title = out.lines().find(|l| l.contains("TokenBurn")).unwrap();
        for part in ["tool:all", "view:daily", "win:all", "cur:USD"] {
            assert!(
                title.contains(part),
                "{part} cut off at 55 columns: {title}"
            );
        }
        // At 45 columns the window chip makes room; the currency still shows.
        let tiny = draw_at(&mut app, 45, 24);
        let title = tiny.lines().find(|l| l.contains("TokenBurn")).unwrap();
        assert!(
            title.contains("cur:USD") && !title.contains("win:"),
            "{title}"
        );
    }
}
