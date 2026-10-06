//! The options popup.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use tui_checkbox::Checkbox;
use tui_slider::{Slider, SliderOrientation};

use super::OptionRow;
use crate::app::App;
use crate::features::theme::UiTheme;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let th = app.theme;
    let popup = centered(area, 56, 12);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled("⚙ ", Style::default().fg(th.accent)),
            Span::styled(
                "Options",
                Style::default().fg(th.accent).add_modifier(Modifier::BOLD),
            ),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(th.accent))
        .style(Style::default().bg(th.surface).fg(th.text));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    // Checkboxes take one line, sliders two (label + value / bar), then the hint.
    let mut constraints: Vec<Constraint> = OptionRow::ALL
        .iter()
        .map(|r| Constraint::Length(if r.is_slider() { 2 } else { 1 }))
        .collect();
    constraints.push(Constraint::Min(1));
    let rows = Layout::vertical(constraints).split(inner);

    for (i, row) in OptionRow::ALL.iter().enumerate() {
        render_row(app, *row, frame, rows[i]);
    }

    frame.render_widget(
        Paragraph::new("↑/↓ select · ␣/Enter toggle · ←/→ adjust · Esc close")
            .style(Style::default().fg(th.muted)),
        rows[OptionRow::ALL.len()],
    );
}

fn render_row(app: &App, row: OptionRow, frame: &mut Frame, area: Rect) {
    let o = &app.options;
    let th = app.theme;
    let selected = o.cursor == row;
    let marker = if selected { "▶ " } else { "  " };
    let [mark, body] = Layout::horizontal([Constraint::Length(2), Constraint::Min(1)]).areas(area);
    frame.render_widget(
        Paragraph::new(marker).style(Style::default().fg(th.accent)),
        mark,
    );

    let label_style = if selected {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    match row {
        OptionRow::AutoRefresh => {
            checkbox(th, "Auto-refresh", o.auto_refresh, label_style, frame, body)
        }
        OptionRow::ShowChart => checkbox(
            th,
            "Show token bar chart",
            o.show_chart,
            label_style,
            frame,
            body,
        ),
        OptionRow::ShowPie => checkbox(
            th,
            "Show share pie chart",
            o.show_pie,
            label_style,
            frame,
            body,
        ),
        OptionRow::HiResPie => checkbox(
            th,
            "High-resolution (braille) pie",
            o.hires_pie,
            label_style,
            frame,
            body,
        ),
        OptionRow::RefreshSecs => slider(
            th,
            "Refresh every (s)",
            &o.refresh_secs,
            selected,
            frame,
            body,
        ),
        OptionRow::TopN => slider(th, "Pie slices (top N)", &o.top_n, selected, frame, body),
    }
}

fn checkbox(
    th: UiTheme,
    label: &str,
    checked: bool,
    label_style: Style,
    frame: &mut Frame,
    area: Rect,
) {
    frame.render_widget(
        Checkbox::new(label, checked)
            .checked_symbol("☑")
            .unchecked_symbol("☐")
            .checkbox_style(Style::default().fg(if checked { th.success } else { th.muted }))
            .label_style(label_style),
        area,
    );
}

fn slider(
    th: UiTheme,
    label: &str,
    state: &tui_slider::SliderState,
    selected: bool,
    frame: &mut Frame,
    area: Rect,
) {
    let color = if selected { th.accent } else { th.muted };
    // tui-slider draws a horizontal slider's label + value on the row *above*
    // the area it is given, so hand it only the bottom row of our 2-row slot.
    let area = Rect {
        y: area.y + 1,
        height: 1,
        ..area
    };
    frame.render_widget(
        Slider::from_state(state)
            .orientation(SliderOrientation::Horizontal)
            .label(label)
            .show_value(true)
            .filled_symbol("━")
            .empty_symbol("─")
            .handle_symbol("●")
            .filled_color(color)
            .handle_color(th.text),
        area,
    );
}

/// A `w`×`h` rectangle centred in `area` (clamped to it).
pub fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_is_clamped_and_centred() {
        let r = centered(Rect::new(0, 0, 100, 40), 50, 10);
        assert_eq!((r.x, r.y, r.width, r.height), (25, 15, 50, 10));
        let r = centered(Rect::new(0, 0, 20, 5), 50, 10);
        assert_eq!((r.width, r.height), (20, 5));
    }
}
