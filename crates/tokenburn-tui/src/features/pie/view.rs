use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use tui_piechart::{PieChart, PieSlice};

use crate::app::App;
use crate::utils::truncate;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let th = app.theme;
    let pie = tokenburn_core::pie(&app.projects, app.options.top_n());
    let (data, metric) = (&pie.slices, pie.metric);
    let title = format!(" Share by project · {} ", metric.label());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(th.border))
        .title(title);

    if data.is_empty() {
        frame.render_widget(
            Paragraph::new("No data.")
                .style(Style::default().fg(th.muted))
                .block(block),
            area,
        );
        return;
    }

    let names: Vec<String> = data.iter().map(|s| truncate(&s.label, 18)).collect();
    let slices: Vec<PieSlice> = data
        .iter()
        .enumerate()
        .map(|(i, s)| PieSlice::new(&names[i], s.value, th.slice(i)))
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
