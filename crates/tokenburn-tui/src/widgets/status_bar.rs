//! Bottom key-hint / status line.

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let hints =
        "q quit · r refresh · t tool · w window · ←/→ or 1-4 view · p projects · ↑/↓ scroll";
    let mut text = hints.to_string();
    if let Some(w) = app.warnings.first() {
        text = format!("⚠ {w}   │   {hints}");
    } else if let Some(s) = &app.status_message {
        text = format!("{s}   │   {hints}");
    }
    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::Black).bg(Color::DarkGray)),
        area,
    );
}
