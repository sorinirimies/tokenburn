//! The savings detail panel: which tool saved the tokens, of what type, and where.

use iced::widget::{column, container, row, scrollable, text};
use iced::{Element, Length};
use tokenburn_core::utils::fmt::{compact, thousands};
use tokenburn_core::{DetailGroup, SavingsDetail};

use crate::message::Message;
use crate::state::TokenBurn;

/// Latest events listed at the bottom.
const RECENT: usize = 50;

/// One table line of a group: `(source, key, tokens, events)` as text.
pub fn group_line(g: &DetailGroup) -> [String; 4] {
    [
        g.source.label().to_string(),
        g.key.clone(),
        format!("{} tok", compact(g.tokens)),
        format!("{} {}", thousands(g.events), g.source.event_noun()),
    ]
}

fn cell<'a>(s: String, w: f32) -> Element<'a, Message> {
    text(s).size(13).width(Length::Fixed(w)).into()
}

fn group_table<'a>(title: &str, groups: Vec<DetailGroup>) -> Element<'a, Message> {
    let mut col = column![text(title.to_string()).size(16).style(text::primary)].spacing(4);
    for g in &groups {
        let [source, key, tokens, events] = group_line(g);
        col = col.push(
            row![
                cell(source, 120.0),
                text(key).size(13).width(Length::Fill),
                cell(tokens, 90.0),
                cell(events, 110.0),
            ]
            .spacing(10),
        );
    }
    col.into()
}

fn recent<'a>(d: &SavingsDetail) -> Element<'a, Message> {
    let mut col = column![text("Latest savings").size(16).style(text::primary)].spacing(4);
    for e in d.events.iter().take(RECENT) {
        let place = if e.origin.is_empty() {
            String::new()
        } else {
            format!(" · {}", tokenburn_core::short_name(&e.origin))
        };
        col = col.push(
            column![
                row![
                    cell(
                        e.ts.with_timezone(&chrono::Local)
                            .format("%m-%d %H:%M")
                            .to_string(),
                        90.0
                    ),
                    cell(e.source.label().to_string(), 120.0),
                    text(format!("{}{place}", e.kind))
                        .size(13)
                        .width(Length::Fill),
                    cell(format!("{} tok", compact(e.tokens)), 90.0),
                ]
                .spacing(10),
                text(e.note.clone()).size(11).style(text::secondary),
            ]
            .spacing(1),
        );
    }
    col.into()
}

pub fn view(state: &TokenBurn) -> Element<'_, Message> {
    let d = &state.detail;
    let body: Element<'_, Message> = if d.is_empty() {
        text("No savings found in this window (rtk, pi-squeeze, pi-context-diet, pi compaction).")
            .style(text::secondary)
            .into()
    } else {
        column![
            group_table("Saved by type", d.by_kind()),
            group_table("Saved by origin", d.by_origin()),
            recent(d),
        ]
        .spacing(20)
        .into()
    };
    scrollable(
        container(
            column![
                text("💾 Savings in detail").size(20).style(text::primary),
                text("Source, type and origin of every saved token. Prompt-cache hits are in the totals, not listed here.")
                    .size(12)
                    .style(text::secondary),
                body,
            ]
            .spacing(12),
        )
        .padding(16),
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokenburn_core::SavingsSource;

    #[test]
    fn a_group_line_names_source_key_tokens_and_events() {
        let g = DetailGroup {
            source: SavingsSource::Rtk,
            key: "git".into(),
            tokens: 12_345,
            events: 7,
        };
        let l = group_line(&g);
        assert_eq!((l[0].as_str(), l[1].as_str()), ("rtk", "git"));
        assert!(l[2].ends_with("tok") && l[3] == "7 commands", "{l:?}");
    }

    #[test]
    fn the_panel_builds_empty_and_with_events() {
        let mut app = TokenBurn::default();
        let _ = view(&app);
        app.detail = tokenburn_core::SavingsDetail {
            events: vec![tokenburn_core::DetailEvent {
                ts: "2026-01-02T00:00:00Z".parse().unwrap(),
                source: SavingsSource::PiSqueeze,
                kind: "search".into(),
                origin: "/home/u/proj".into(),
                tokens: 50,
                note: "bash output shortened".into(),
            }],
        };
        let _ = view(&app);
    }
}
