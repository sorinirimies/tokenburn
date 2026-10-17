//! Title bar with the grand total and the active query.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use tokenburn_core::{CardKind, LayoutClass};

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
        // The version yields to the chips on very narrow terminals.
        Span::styled(
            if area.width >= 64 {
                format!("{} ", tokenburn_core::appinfo::version_label())
            } else {
                " ".to_string()
            },
            Style::default().fg(th.muted),
        ),
        chip(th, "tool", q.tool.as_str()),
        chip(th, "view", q.bucket.label()),
    ];
    if !narrow {
        // The metric the headline figures and the chart show (`m` cycles it).
        title_spans.insert(2, chip(th, "metric", app.metric.label()));
        title_spans.push(chip(th, "window", q.window.label()));
    } else if area.width >= 52 {
        title_spans.push(chip(th, "win", q.window.label()));
    }
    // The currency is part of what the cost means, so it stays on every width.
    title_spans.push(chip(
        th,
        "cur",
        &format!("{}{}", app.currency.code, app.currency.approx_mark()),
    ));
    title_spans.push(filter_chips(app));
    // The theme is always the last chip.
    if !class.is_narrow() {
        title_spans.push(chip(
            th,
            "theme",
            if app.follow_system {
                tokenburn_core::SYSTEM_THEME_NAME
            } else {
                tokenburn_core::THEME_NAMES[app.theme_index]
            },
        ));
    }
    title_spans.push(Span::styled(
        if app.is_loading { " loading…" } else { "" },
        Style::default().fg(th.muted),
    ));
    let title = Line::from(title_spans);
    let title_w = title.width();

    // The headline figures follow the metric: token counts, spend, or savings. As
    // many as fit the line.
    let mut totals_spans = Vec::new();
    let mut used = 0usize;
    let room = area.width.saturating_sub(2) as usize;
    let max_cards = match class {
        LayoutClass::Narrow => 2,
        LayoutClass::Medium => 4,
        LayoutClass::Wide => usize::MAX,
    };
    for card in tokenburn_core::cards(app.metric, &t, app.savings(), &app.currency)
        .into_iter()
        .take(max_cards)
    {
        let span = match card.kind {
            CardKind::Plain => stat(th, &card.label, card.value),
            CardKind::Cost => badge(th.success, th.bg, &card.label, &card.value),
            CardKind::Saved => badge(th.accent, th.bg, &card.label, &card.value),
        };
        used += span.width();
        if used > room {
            break;
        }
        totals_spans.push(span);
    }
    let totals = Line::from(totals_spans);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(th.border));
    frame.render_widget(Paragraph::new(vec![title, totals]).block(block), area);
    // The menu, top right of the title line: settings and about (not on narrow
    // terminals, where the status bar lists the keys).
    const MENU_W: usize = 22;
    if !class.is_narrow() && title_w + MENU_W + 2 <= area.width as usize {
        let menu = Line::from(vec![
            Span::styled(" o ", Style::default().fg(th.bg).bg(th.accent)),
            Span::styled(" Settings ", Style::default().fg(th.text)),
            Span::styled(" i ", Style::default().fg(th.bg).bg(th.accent)),
            Span::styled(" About ", Style::default().fg(th.text)),
        ]);
        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: 1,
        };
        frame.render_widget(Paragraph::new(menu).alignment(Alignment::Right), inner);
    }
}

fn chip(th: UiTheme, label: &str, value: &str) -> Span<'static> {
    Span::styled(format!(" {label}:{value} "), Style::default().fg(th.accent))
}

/// A highlighted figure (spend or savings).
fn badge(
    bg: ratatui::style::Color,
    fg: ratatui::style::Color,
    label: &str,
    value: &str,
) -> Span<'static> {
    Span::styled(
        format!(" {label} {value} "),
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    )
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
