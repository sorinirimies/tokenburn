//! The headline cards above the charts. What they show follows the chart's
//! [`Metric`]: token counts, spend, or what was saved.

use super::chart::Metric;
use crate::features::currency::Currency;
use crate::features::savings::Savings;
use crate::features::usage::Summary;
use crate::utils::fmt::{compact, percent, thousands};

/// How a card is styled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardKind {
    Plain,
    Cost,
    Saved,
}

/// One headline figure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub label: String,
    pub value: String,
    pub kind: CardKind,
}

fn card(label: &str, value: String, kind: CardKind) -> Card {
    Card {
        label: label.to_string(),
        value,
        kind,
    }
}

/// The cards for `metric`:
/// - tokens: turns and the token counts,
/// - cost: turns, spend and what a turn / a million tokens cost,
/// - saved: money and tokens saved, their share of the would-be cost, and one
///   card per source.
pub fn cards(metric: Metric, total: &Summary, savings: &Savings, currency: &Currency) -> Vec<Card> {
    let approx = currency.approx_mark();
    match metric {
        Metric::Tokens => vec![
            card("turns", thousands(total.turns), CardKind::Plain),
            card("total tokens", compact(total.total_tokens), CardKind::Plain),
            card("input", compact(total.input), CardKind::Plain),
            card("output", compact(total.output), CardKind::Plain),
            card("cache read", compact(total.cache_read), CardKind::Plain),
            card("cache write", compact(total.cache_write), CardKind::Plain),
        ],
        Metric::Cost => {
            let per_turn = if total.turns > 0 {
                total.cost_usd / total.turns as f64
            } else {
                0.0
            };
            let per_mtok = if total.total_tokens > 0 {
                total.cost_usd / (total.total_tokens as f64 / 1e6)
            } else {
                0.0
            };
            vec![
                card("turns", thousands(total.turns), CardKind::Plain),
                card(
                    &format!("cost{approx}"),
                    currency.format(total.cost_usd),
                    CardKind::Cost,
                ),
                card(
                    &format!("per turn{approx}"),
                    currency.format(per_turn),
                    CardKind::Cost,
                ),
                card(
                    &format!("per 1M tokens{approx}"),
                    currency.format(per_mtok),
                    CardKind::Cost,
                ),
            ]
        }
        Metric::Saved => {
            let mut out = vec![
                card(
                    &format!("saved{approx}"),
                    currency.format(savings.usd),
                    CardKind::Saved,
                ),
                card("tokens saved", compact(savings.tokens), CardKind::Saved),
            ];
            if let Some(p) = savings.percent_of_would_be(total.cost_usd) {
                out.push(card("of would-be cost", percent(p), CardKind::Saved));
            }
            for s in &savings.sources {
                out.push(card(
                    s.source.label(),
                    currency.format(s.usd),
                    CardKind::Saved,
                ));
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total() -> Summary {
        Summary {
            turns: 4,
            input: 100,
            output: 50,
            cache_read: 2_000_000,
            cache_write: 10,
            total_tokens: 2_000_160,
            cost_usd: 8.0,
        }
    }

    fn saved() -> Savings {
        Savings::build(
            &[],
            &crate::ExternalSavings {
                rtk_tokens: 1_000_000,
                rtk_commands: 3,
                ..Default::default()
            },
            2.0,
        )
    }

    fn labels(c: &[Card]) -> Vec<&str> {
        c.iter().map(|c| c.label.as_str()).collect()
    }

    #[test]
    fn each_metric_shows_different_cards() {
        let usd = Currency::USD;
        let t = cards(Metric::Tokens, &total(), &saved(), &usd);
        let c = cards(Metric::Cost, &total(), &saved(), &usd);
        let s = cards(Metric::Saved, &total(), &saved(), &usd);
        assert_eq!(
            labels(&t),
            [
                "turns",
                "total tokens",
                "input",
                "output",
                "cache read",
                "cache write"
            ]
        );
        assert_eq!(labels(&c), ["turns", "cost", "per turn", "per 1M tokens"]);
        assert_eq!(labels(&s)[..2], ["saved", "tokens saved"]);
        assert!(t.iter().all(|c| c.kind == CardKind::Plain));
        assert!(s.iter().all(|c| c.kind == CardKind::Saved));
    }

    #[test]
    fn cost_cards_divide_by_turns_and_tokens() {
        let c = cards(Metric::Cost, &total(), &Savings::default(), &Currency::USD);
        assert_eq!(c[1].value, "$8.00");
        assert_eq!(c[2].value, "$2.00", "8 over 4 turns");
        assert_eq!(c[3].value, "$4.00", "8 over ~2M tokens");
        let none = cards(
            Metric::Cost,
            &Summary::default(),
            &Savings::default(),
            &Currency::USD,
        );
        assert_eq!(none[2].value, "$0.00", "no division by zero");
        assert_eq!(none[3].value, "$0.00");
    }

    #[test]
    fn saved_cards_list_money_tokens_share_and_each_source() {
        let s = cards(Metric::Saved, &total(), &saved(), &Currency::USD);
        assert_eq!(s[0].value, "$2.00");
        assert_eq!(s[1].value, "1.00M");
        assert_eq!(
            labels(&s),
            ["saved", "tokens saved", "of would-be cost", "rtk"]
        );
        assert_eq!(s[2].value, percent(0.2));
        assert_eq!(s[3].value, "$2.00");
        // Nothing saved: just zeros, no share and no sources.
        let none = cards(
            Metric::Saved,
            &Summary::default(),
            &Savings::default(),
            &Currency::USD,
        );
        assert_eq!(labels(&none), ["saved", "tokens saved"]);
    }
}
