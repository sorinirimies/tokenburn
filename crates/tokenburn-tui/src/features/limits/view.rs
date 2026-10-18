//! One line of limit progress above the report.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tokenburn_core::{LayoutClass, Level, LimitStatus};

use crate::app::App;

/// `▰▰▰▰▱▱▱▱` for `ratio` (clamped to 0..=1).
pub fn bar(ratio: f64, cells: usize) -> String {
    let filled = ((ratio.clamp(0.0, 1.0) * cells as f64).round() as usize).min(cells);
    format!("{}{}", "▰".repeat(filled), "▱".repeat(cells - filled))
}

fn style(app: &App, level: Level) -> Style {
    let t = app.theme;
    let c = match level {
        Level::Ok => t.success,
        Level::Warn => t.warning,
        Level::Over => t.error,
    };
    let s = Style::default().fg(c);
    if level == Level::Ok {
        s
    } else {
        s.add_modifier(Modifier::BOLD)
    }
}

fn mark(level: Level) -> &'static str {
    match level {
        Level::Ok => "",
        Level::Warn => "⚠ ",
        Level::Over => "🚨 ",
    }
}

/// The strip: one entry per limit, as many as fit `width`.
pub fn line(app: &App, class: LayoutClass, width: u16) -> Line<'static> {
    let mut spans = vec![Span::styled(
        " limits ",
        Style::default().fg(app.theme.muted),
    )];
    let mut used = spans[0].width();
    for s in &app.limit_statuses {
        let text = entry(app, s, class);
        let w = text.chars().count() + 3;
        if used + w > width as usize {
            break;
        }
        used += w;
        spans.push(Span::styled(text, style(app, s.level)));
        spans.push(Span::styled(" │ ", Style::default().fg(app.theme.border)));
    }
    spans.pop();
    Line::from(spans)
}

fn entry(app: &App, s: &LimitStatus, class: LayoutClass) -> String {
    let money = |v| app.currency.format(v);
    let label = match class {
        LayoutClass::Narrow => s.period.label().chars().next().unwrap_or('?').to_string(),
        _ => s.period.label().to_string(),
    };
    let bar = if class == LayoutClass::Narrow {
        String::new()
    } else {
        format!("{} ", bar(s.ratio(), 8))
    };
    format!(
        "{}{label} {bar}{}/{} {}%",
        mark(s.level),
        money(s.spent_usd),
        money(s.limit_usd),
        s.percent()
    )
}

pub fn render(app: &App, frame: &mut Frame, area: Rect, class: LayoutClass) {
    frame.render_widget(Paragraph::new(line(app, class, area.width)), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokenburn_core::Period;

    #[test]
    fn the_bar_fills_proportionally_and_clamps() {
        assert_eq!(bar(0.0, 4), "▱▱▱▱");
        assert_eq!(bar(0.5, 4), "▰▰▱▱");
        assert_eq!(bar(1.0, 4), "▰▰▰▰");
        assert_eq!(bar(7.0, 4), "▰▰▰▰");
        assert_eq!(bar(-1.0, 4), "▱▱▱▱");
    }

    fn status(period: Period, spent: f64, limit: f64, level: Level) -> LimitStatus {
        LimitStatus {
            period,
            limit_usd: limit,
            spent_usd: spent,
            level,
        }
    }

    #[test]
    fn entries_show_the_mark_amounts_and_percent() {
        let mut app = App::new();
        app.limit_statuses = vec![
            status(Period::Day, 12.4, 20.0, Level::Ok),
            status(Period::Week, 90.0, 100.0, Level::Warn),
            status(Period::Month, 400.0, 300.0, Level::Over),
        ];
        let text: String = line(&app, LayoutClass::Wide, 200)
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert!(text.contains("daily ▰▰▰▰▰▱▱▱ $12.40/$20.00 62%"), "{text}");
        assert!(text.contains("⚠ weekly") && text.contains("90%"), "{text}");
        assert!(
            text.contains("🚨 monthly") && text.contains("133%"),
            "{text}"
        );
        // Narrow: just the initial, no bar, and only what fits.
        let narrow: String = line(&app, LayoutClass::Narrow, 30)
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert!(
            narrow.contains("d $12.40/$20.00 62%") && !narrow.contains('▰'),
            "{narrow}"
        );
        assert!(
            !narrow.contains("monthly") && narrow.chars().count() <= 30,
            "{narrow}"
        );
    }
}
