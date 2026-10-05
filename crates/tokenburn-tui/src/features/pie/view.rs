use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use tui_piechart::{PieChart, PieSlice};

use super::pie_data;
use crate::app::App;
use crate::utils::truncate;

const COLORS: [Color; 8] = [
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Green,
    Color::LightRed,
    Color::LightBlue,
    Color::LightMagenta,
    Color::Gray,
];

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let (data, metric) = pie_data(&app.projects, app.options.top_n());
    let title = format!(" Share by project · {} ", metric.label());
    let block = Block::default().borders(Borders::ALL).title(title);

    if data.is_empty() {
        frame.render_widget(Paragraph::new("No data.").block(block), area);
        return;
    }

    let names: Vec<String> = data.iter().map(|(n, _)| truncate(n, 18)).collect();
    let slices: Vec<PieSlice> = data
        .iter()
        .enumerate()
        .map(|(i, (_, v))| PieSlice::new(&names[i], *v, COLORS[i % COLORS.len()]))
        .collect();

    frame.render_widget(
        PieChart::new(slices)
            .block(block)
            .show_legend(true)
            .show_percentages(true)
            .high_resolution(app.options.hires_pie),
        area,
    );
}
