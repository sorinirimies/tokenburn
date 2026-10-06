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

    let hints = "q quit · r refresh · t tool · w window · ←/→ 1-4 view · p projects · o options · T theme · [ ] cycle";
    let text = if let Some(w) = app.warnings.first() {
        format!("⚠ {w}   │   {hints}")
    } else if let Some(s) = &app.status_message {
        format!("{s}   │   {hints}")
    } else {
        hints.to_string()
    };
    frame.render_widget(Paragraph::new(text).style(bar), right);
}
