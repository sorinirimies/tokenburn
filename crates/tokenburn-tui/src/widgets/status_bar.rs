//! Bottom status line: loading spinner, message and key hints.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_spinner::{FluxFrames, FluxSpinner};

use crate::app::App;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let bar = Style::default().fg(app.theme.bg).bg(app.theme.accent);
    // [3 cols: spinner] [rest: message + hints]
    let [left, right] = Layout::horizontal([Constraint::Length(3), Constraint::Min(0)]).areas(area);

    frame.render_widget(Paragraph::new("").style(bar), left);

    // Animated spinner (tui-spinner) while a background load is in flight.
    if app.is_loading {
        let cell = Rect {
            x: left.x + 1,
            y: left.y,
            width: 1,
            height: 1,
        };
        frame.render_widget(
            FluxSpinner::new(app.tick_count)
                .frames(FluxFrames::CORNERS)
                .color(app.theme.bg),
            cell,
        );
    }

    let hints = "q quit · / filter · f tools · t tool · w window · ←/→ view · p projects · o options · T theme";
    if app.filter_editing {
        // The text box: show the cursor and what the keys do.
        let line = format!(
            " / {}█    Enter keep · Esc clear · Ctrl-U wipe · \"-term\" excludes",
            app.filter.text
        );
        frame.render_widget(Paragraph::new(line).style(bar), area);
        return;
    }
    let text = if let Some(w) = app.warnings.first() {
        format!("⚠ {w}   │   {hints}")
    } else if let Some(s) = &app.status_message {
        format!("{s}   │   {hints}")
    } else {
        hints.to_string()
    };
    frame.render_widget(Paragraph::new(text).style(bar), right);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn draw(app: &App) -> String {
        let mut term = Terminal::new(TestBackend::new(140, 1)).unwrap();
        term.draw(|f| render(app, f, f.area())).unwrap();
        term.backend().to_string()
    }

    #[test]
    fn idle_shows_the_key_hints() {
        let out = draw(&App::new());
        assert!(out.contains("q quit") && out.contains("/ filter"), "{out}");
    }

    #[test]
    fn a_status_message_precedes_the_hints() {
        let mut app = App::new();
        app.status_message = Some("Updated 10:00:00".into());
        let out = draw(&app);
        assert!(out.contains("Updated 10:00:00") && out.contains("q quit"));
    }

    #[test]
    fn a_warning_wins_over_the_status_message() {
        let mut app = App::new();
        app.status_message = Some("Updated".into());
        app.warnings = vec!["codex: unreadable".into()];
        let out = draw(&app);
        assert!(out.contains("⚠ codex: unreadable"));
        assert!(!out.contains("Updated"));
    }

    #[test]
    fn loading_draws_the_spinner_in_the_left_gutter() {
        let mut app = App::new();
        app.is_loading = true;
        let mut seen = std::collections::HashSet::new();
        for t in 0..8 {
            app.tick_count = t;
            let out = draw(&app);
            seen.insert(out.chars().skip(1).take(3).collect::<String>());
        }
        assert!(seen.len() > 1, "the spinner animates: {seen:?}");
        app.is_loading = false;
        assert_eq!(
            draw(&app)
                .chars()
                .skip(1)
                .take(3)
                .collect::<String>()
                .trim(),
            ""
        );
    }

    #[test]
    fn filter_editing_shows_the_text_box_and_its_keys() {
        let mut app = App::new();
        app.filter_editing = true;
        app.filter.text = "pi -zed".into();
        let out = draw(&app);
        assert!(out.contains("/ pi -zed█"), "{out}");
        assert!(out.contains("Enter keep") && out.contains("Esc clear"));
        assert!(!out.contains("q quit"), "hints give way to the editor");
    }
}
