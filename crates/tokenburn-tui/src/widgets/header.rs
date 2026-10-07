//! Title bar with the grand total and the active query.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use tokenburn_core::utils::fmt::compact;
use tokenburn_core::LayoutClass;

use crate::app::App;
use crate::features::theme::UiTheme;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = app.total();
    let q = app.query;
    let th = app.theme;
    // Wide: everything. Medium: no cache totals. Narrow: only what you must see, the
    // theme and currency are one keypress away.
    let class = LayoutClass::from_cols(area.width);

    // Narrow: a terse title and short chip labels so the line fits ~55 columns, and
    // below ~50 the window chip goes too.
    let narrow = class.is_narrow();
    let mut title_spans = vec![
        Span::styled(
            if narrow {
                " TokenBurn "
            } else {
                " 🔥 TokenBurn "
            },
            Style::default().fg(th.warning).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        chip(th, "tool", q.tool.as_str()),
        chip(th, "view", q.bucket.label()),
    ];
    if !narrow {
        title_spans.push(chip(th, "window", q.window.label()));
    } else if area.width >= 52 {
        title_spans.push(chip(th, "win", q.window.label()));
    }
    if !class.is_narrow() {
        title_spans.push(chip(
            th,
            "theme",
            tokenburn_core::THEME_NAMES[app.theme_index],
        ));
    }
    // The currency is part of what the cost means, so it stays on every width.
    title_spans.push(chip(
        th,
        "cur",
        &format!("{}{}", app.currency.code, app.currency.approx_mark()),
    ));
    title_spans.push(filter_chips(app));
    title_spans.push(Span::styled(
        if app.is_loading { " loading…" } else { "" },
        Style::default().fg(th.muted),
    ));
    let title = Line::from(title_spans);

    let mut totals_spans = vec![
        stat(th, "turns", t.turns.to_string()),
        stat(th, "total", compact(t.total_tokens)),
    ];
    if !class.is_narrow() {
        totals_spans.push(stat(th, "in", compact(t.input)));
        totals_spans.push(stat(th, "out", compact(t.output)));
    }
    if class.is_wide() {
        totals_spans.push(stat(
            th,
            "cache r/w",
            format!("{}/{}", compact(t.cache_read), compact(t.cache_write)),
        ));
    }
    totals_spans.push(Span::styled(
        format!(" {} ", app.currency.format(t.cost_usd)),
        Style::default()
            .fg(th.bg)
            .bg(th.success)
            .add_modifier(Modifier::BOLD),
    ));
    let totals = Line::from(totals_spans);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(th.border));
    frame.render_widget(Paragraph::new(vec![title, totals]).block(block), area);
}

fn chip(th: UiTheme, label: &str, value: &str) -> Span<'static> {
    Span::styled(format!(" {label}:{value} "), Style::default().fg(th.accent))
}

fn stat(th: UiTheme, label: &str, value: String) -> Span<'static> {
    Span::styled(format!(" {label} {value} │"), Style::default().fg(th.text))
}

/// `filter:"text"  tools:3/8  412/3,029` — empty when nothing is filtered.
fn filter_chips(app: &App) -> Span<'static> {
    let th = app.theme;
    if app.filter.is_empty() {
        return Span::raw("");
    }
    let mut parts = Vec::new();
    if !app.filter.text.trim().is_empty() {
        parts.push(format!("filter:\"{}\"", app.filter.text.trim()));
    }
    if app.filter.tools.is_some() {
        parts.push(format!(
            "tools:{}/{}",
            app.filter.tool_count(),
            tokenburn_core::Tool::ALL.len()
        ));
    }
    parts.push(format!("{}/{} rows", app.rows_len, app.all_rows.len()));
    Span::styled(
        format!(" ⌕ {} ", parts.join("  ")),
        Style::default()
            .fg(th.bg)
            .bg(th.warning)
            .add_modifier(Modifier::BOLD),
    )
}
