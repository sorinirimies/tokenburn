//! "Saved" panel: what prompt caching, rtk and pi compaction saved, per source.

use iced::widget::{column, container, row, text};
use iced::{Element, Length};
use tokenburn_core::utils::fmt::{compact, percent, thousands};
use tokenburn_core::LayoutClass;

use crate::message::Message;
use crate::state::TokenBurn;

/// One printed row of the panel: `(source, tokens, money, events)`.
pub type Line = (String, String, String, String);

/// The panel's rows in display order, ending with the total. Empty when nothing
/// was saved. Pure, so tests read the numbers without building widgets.
pub fn lines(state: &TokenBurn) -> Vec<Line> {
    let sv = &state.savings;
    if sv.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<Line> = sv
        .sources
        .iter()
        .map(|s| {
            (
                s.source.label().to_string(),
                format!("{} tok", compact(s.tokens)),
                state.currency.format(s.usd),
                format!("{} {}", thousands(s.events), s.source.event_noun()),
            )
        })
        .collect();
    out.push((
        "total".to_string(),
        format!("{} tok", compact(sv.tokens)),
        state.currency.format(sv.usd),
        sv.percent_of_would_be(state.total.cost_usd)
            .map(|p| format!("{} of the would-be cost", percent(p)))
            .unwrap_or_default(),
    ));
    out
}

/// How many buckets the panel lists (newest first).
pub const MAX_BUCKET_LINES: usize = 12;

/// Savings per chart bucket (minute, hour, day or month — whatever the chart
/// uses), newest first: `(bucket, tokens, money, per-source split)`.
pub fn bucket_lines(state: &TokenBurn) -> Vec<Line> {
    state
        .savings_by_bucket
        .iter()
        .rev()
        .take(MAX_BUCKET_LINES)
        .map(|b| {
            let split = b
                .savings
                .sources
                .iter()
                .map(|s| format!("{} {}", s.source.label(), state.currency.format(s.usd)))
                .collect::<Vec<_>>()
                .join(" · ");
            (
                b.label.clone(),
                format!("{} tok", compact(b.savings.tokens)),
                state.currency.format(b.savings.usd),
                split,
            )
        })
        .collect()
}

/// The heading above the per-bucket list, e.g. `Saved per daily bucket`.
pub fn bucket_heading(state: &TokenBurn) -> String {
    let name = match tokenburn_core::chart_bucket(&state.query) {
        tokenburn_core::Bucket::Minute => "minute",
        tokenburn_core::Bucket::Hour => "hour",
        tokenburn_core::Bucket::Day => "day",
        _ => "month",
    };
    format!("Saved per {name}")
}

/// The heading: says the figures are estimates and at what price.
pub fn heading(state: &TokenBurn) -> String {
    format!(
        "Saved (estimate · {}/M input tokens)",
        state.currency.format(state.input_price)
    )
}

/// The panel, or nothing when there is nothing to show.
pub fn view(state: &TokenBurn, layout: LayoutClass) -> Option<Element<'_, Message>> {
    let rows = lines(state);
    if rows.is_empty() {
        return None;
    }
    // Only with the "saved" chart metric; the other metrics show their own figures.
    if state.chart_metric != tokenburn_core::Metric::Saved {
        return None;
    }
    let size = if layout.is_narrow() { 12 } else { 14 };
    let last = rows.len() - 1;
    let buckets = bucket_lines(state);
    let mut body = column![text(heading(state)).size(12).style(text::secondary)].spacing(4);
    for (i, (label, tokens, money, events)) in rows.into_iter().enumerate() {
        let line = row![
            text(label).size(size).width(Length::FillPortion(3)),
            text(tokens).size(size).width(Length::FillPortion(3)),
            text(money)
                .size(size)
                .style(text::success)
                .width(Length::FillPortion(3)),
            text(events)
                .size(size)
                .style(text::secondary)
                .width(Length::FillPortion(5)),
        ]
        .spacing(8);
        body = body.push(if i == last {
            line.padding([4, 0])
        } else {
            line
        });
    }
    if !buckets.is_empty() {
        body = body.push(text(bucket_heading(state)).size(12).style(text::secondary));
        for (label, tokens, money, split) in buckets {
            body = body.push(
                row![
                    text(label).size(size).width(Length::FillPortion(3)),
                    text(tokens).size(size).width(Length::FillPortion(3)),
                    text(money)
                        .size(size)
                        .style(text::success)
                        .width(Length::FillPortion(3)),
                    text(split)
                        .size(size)
                        .style(text::secondary)
                        .width(Length::FillPortion(5)),
                ]
                .spacing(8),
            );
        }
    }
    Some(
        container(body)
            .padding(10)
            .width(Length::Fill)
            .style(container::bordered_box)
            .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokenburn_core::{Collected, ExternalSavings, Row, Tool};

    fn at(ts: &str) -> chrono::DateTime<chrono::Utc> {
        ts.parse().unwrap()
    }

    fn row(project: &str, ts: &str, cache_read: u64) -> Row {
        Row {
            tool: Tool::Pi,
            project: project.into(),
            id: "x".into(),
            ts: ts.parse().unwrap(),
            input: 10,
            output: 5,
            cache_read,
            cache_write: 0,
            cost: 0.5,
        }
    }

    /// Two projects on two days; `alpha` read 1M cached tokens, `beta` none.
    /// $10/M, plus 1M tokens from rtk and from pi compaction.
    fn loaded() -> TokenBurn {
        let mut app = TokenBurn::new();
        app.input_price = 10.0;
        app.apply(Collected {
            rows: vec![
                row("alpha", "2026-05-01T10:00:00Z", 1_000_000),
                row("beta", "2026-05-02T10:00:00Z", 0),
            ],
            external: ExternalSavings {
                rtk_tokens: 1_000_000,
                rtk_commands: 4,
                compaction_tokens: 1_000_000,
                compactions: 1,
                // rtk on the second day, the compaction on the first.
                rtk_events: vec![(at("2026-05-02T10:00:00Z"), 1_000_000, 4)],
                compaction_events: vec![(at("2026-05-01T10:00:00Z"), 1_000_000)],
            },
            ..Default::default()
        });
        app
    }

    #[test]
    fn one_line_per_source_then_the_total() {
        let l = lines(&loaded());
        let names: Vec<&str> = l.iter().map(|r| r.0.as_str()).collect();
        assert_eq!(names, ["Prompt cache", "rtk", "pi compaction", "total"]);
        assert_eq!(l[0].2, "$9.00", "1M × $10 × 0.9");
        assert_eq!(l[1].3, "4 commands");
        assert_eq!(l[3].1, "3.00M tok");
        assert_eq!(l[3].2, "$29.00");
        assert!(l[3].3.starts_with("97% of the would-be cost"), "{}", l[3].3);
    }

    #[test]
    fn selecting_a_project_keeps_only_that_projects_cache_saving() {
        let mut app = loaded();
        let key = |app: &TokenBurn, name: &str| {
            app.pie
                .slices
                .iter()
                .find_map(|s| s.key.clone().filter(|k| k.ends_with(name)))
                .unwrap()
        };
        let beta = key(&app, "beta");
        app.click_project(&beta);
        assert!(
            lines(&app).is_empty(),
            "beta has no cache hits; rtk and compactions are not per project"
        );
        let alpha = key(&app, "alpha");
        app.click_project(&alpha);
        let l = lines(&app);
        assert_eq!(l.len(), 2, "cache + total: {l:?}");
        assert_eq!(l.last().unwrap().2, "$9.00");
        app.clear_selection();
        assert_eq!(lines(&app).last().unwrap().2, "$29.00");
    }

    #[test]
    fn selecting_a_bucket_narrows_every_source_to_that_interval() {
        use tokenburn_core::{bucket_label_at, Bucket};
        let mut app = loaded();
        let day1 = bucket_label_at(at("2026-05-01T10:00:00Z"), Bucket::Day);
        let day2 = bucket_label_at(at("2026-05-02T10:00:00Z"), Bucket::Day);
        app.click_bucket(&day1);
        let l = lines(&app);
        let names: Vec<&str> = l.iter().map(|r| r.0.as_str()).collect();
        assert_eq!(
            names,
            ["Prompt cache", "pi compaction", "total"],
            "no rtk that day"
        );
        assert_eq!(l.last().unwrap().2, "$19.00", "9 cache + 10 compaction");
        app.click_bucket(&day1); // toggles off
        app.click_bucket(&day2);
        let l = lines(&app);
        let names: Vec<&str> = l.iter().map(|r| r.0.as_str()).collect();
        assert_eq!(names, ["rtk", "total"]);
        assert_eq!(l.last().unwrap().2, "$10.00");
    }

    #[test]
    fn savings_are_listed_per_bucket_newest_first() {
        let app = loaded();
        let l = bucket_lines(&app);
        assert_eq!(l.len(), 2);
        assert!(l[0].0 > l[1].0, "newest first: {l:?}");
        assert_eq!(l[0].2, "$10.00", "day 2: rtk");
        assert_eq!(l[0].3, "rtk $10.00");
        assert_eq!(l[1].2, "$19.00");
        assert_eq!(l[1].3, "Prompt cache $9.00 · pi compaction $10.00");
        assert_eq!(bucket_heading(&app), "Saved per day");
    }

    #[test]
    fn the_per_bucket_granularity_follows_the_bucket_filter() {
        let mut app = loaded();
        app.query.bucket = tokenburn_core::Bucket::Minute;
        app.recompute();
        assert_eq!(bucket_heading(&app), "Saved per minute");
        let l = bucket_lines(&app);
        assert_eq!(
            l.len(),
            2,
            "day 1 10:00 (cache + compaction) and day 2 10:00 (rtk): {l:?}"
        );
        assert!(
            l.iter().all(|r| r.0.ends_with(":00") && r.0.len() == 16),
            "{l:?}"
        );
        app.query.bucket = tokenburn_core::Bucket::Month;
        app.recompute();
        let l = bucket_lines(&app);
        assert_eq!(l.len(), 1, "everything is in May");
        assert_eq!(l[0].2, "$29.00");
        assert_eq!(bucket_heading(&app), "Saved per month");
    }

    #[test]
    fn money_follows_the_currency_and_the_price() {
        let mut app = loaded();
        app.currency = tokenburn_core::currency_by_code("EUR").unwrap();
        assert_eq!(lines(&app).last().unwrap().2, "€26.68");
        assert!(
            heading(&app).contains("€9.20/M input tokens"),
            "{}",
            heading(&app)
        );
        app.input_price = 20.0;
        app.recompute();
        assert_eq!(lines(&app)[0].2, "€33.12".replace("33.12", "16.56"));
    }

    #[test]
    fn nothing_saved_means_no_panel_and_no_card() {
        let mut app = TokenBurn::new();
        assert!(lines(&app).is_empty());
        for layout in [LayoutClass::Wide, LayoutClass::Medium, LayoutClass::Narrow] {
            assert!(view(&app, layout).is_none());
            let _ = app.view_for(layout);
        }
        app.apply(Collected {
            rows: vec![row("a", "2026-05-01T10:00:00Z", 0)],
            ..Default::default()
        });
        assert!(lines(&app).is_empty());
    }

    #[test]
    fn the_panel_builds_at_every_layout_and_only_for_the_saved_metric() {
        use tokenburn_core::Metric;
        let mut app = loaded();
        for layout in [LayoutClass::Wide, LayoutClass::Medium, LayoutClass::Narrow] {
            for metric in Metric::ALL {
                app.chart_metric = metric;
                assert_eq!(
                    view(&app, layout).is_some(),
                    metric == Metric::Saved,
                    "{metric:?}"
                );
                let _ = app.view_for(layout);
            }
        }
        let _ = app.view();
    }

    #[test]
    fn nothing_saved_shows_no_panel() {
        let mut app = loaded();
        app.chart_metric = tokenburn_core::Metric::Saved;
        app.savings = tokenburn_core::Savings::default();
        assert!(view(&app, LayoutClass::Wide).is_none());
    }

    #[test]
    fn the_chart_can_show_saved_money_per_bucket() {
        use tokenburn_core::Metric;
        let mut app = loaded();
        let _ = app.update(Message::MetricSelected(Metric::Saved));
        assert_eq!(app.chart_metric, Metric::Saved);
        // Day 1: 1M cached tokens at $10/M, 90 % saved ($9), plus a $10 compaction.
        let first = app.series.bucket_total(Metric::Saved, 0);
        assert!((first - 19.0).abs() < 1e-9, "{first}");
        assert_eq!(app.series.bucket_total(Metric::Saved, 1), 0.0);
        assert_eq!(
            crate::features::report::view::chart_title(Metric::Saved),
            "Saved over time (cache + compaction)"
        );
        for layout in [LayoutClass::Wide, LayoutClass::Medium, LayoutClass::Narrow] {
            let _ = app.view_for(layout);
        }
    }
}
