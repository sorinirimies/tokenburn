use serde::{Deserialize, Serialize};

/// Default price of one million input tokens in US dollars (a mid-range model),
/// used until the user sets `input_price_per_mtok` in `settings.json`.
pub const DEFAULT_INPUT_PRICE_PER_MTOK: f64 = 3.0;

/// A prompt-cache read costs about a tenth of a fresh input token.
pub const CACHE_READ_PRICE_RATIO: f64 = 0.1;

/// Where a saving came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SavingsSource {
    /// Provider-side prompt cache hits.
    PromptCache,
    /// rtk output filtering.
    Rtk,
    /// pi context compaction.
    PiCompaction,
    /// pi-context-diet eliding stale tool results.
    PiContextDiet,
    /// pi-squeeze compacting bash output.
    PiSqueeze,
}

impl SavingsSource {
    /// Every source, in display order.
    pub const ALL: [SavingsSource; 5] = [
        SavingsSource::PromptCache,
        SavingsSource::Rtk,
        SavingsSource::PiCompaction,
        SavingsSource::PiContextDiet,
        SavingsSource::PiSqueeze,
    ];

    /// Short machine name.
    pub fn as_str(self) -> &'static str {
        match self {
            SavingsSource::PromptCache => "prompt_cache",
            SavingsSource::Rtk => "rtk",
            SavingsSource::PiCompaction => "pi_compaction",
            SavingsSource::PiContextDiet => "pi_context_diet",
            SavingsSource::PiSqueeze => "pi_squeeze",
        }
    }

    /// Human-readable name.
    pub fn label(self) -> &'static str {
        match self {
            SavingsSource::PromptCache => "Prompt cache",
            SavingsSource::Rtk => "rtk",
            SavingsSource::PiCompaction => "pi compaction",
            SavingsSource::PiContextDiet => "pi context-diet",
            SavingsSource::PiSqueeze => "pi squeeze",
        }
    }

    /// What one "event" of this source is.
    pub fn event_noun(self) -> &'static str {
        match self {
            SavingsSource::PromptCache => "turns",
            SavingsSource::Rtk => "commands",
            SavingsSource::PiCompaction => "compactions",
            SavingsSource::PiContextDiet => "requests",
            SavingsSource::PiSqueeze => "outputs",
        }
    }
}

/// Savings reported by sources that are not part of the usage rows: rtk's
/// history database, pi's compaction entries and the savings logs of
/// pi-context-diet and pi-squeeze. Filled in by [`crate::collect`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalSavings {
    /// Tokens rtk removed from command output.
    pub rtk_tokens: u64,
    /// Commands rtk filtered.
    pub rtk_commands: u64,
    /// Estimated tokens pi's compactions removed from the context.
    pub compaction_tokens: u64,
    /// Number of pi compactions.
    pub compactions: u64,
    /// rtk's savings per minute: `(minute, tokens, commands)`, for the
    /// savings-by-interval views.
    #[serde(skip)]
    pub rtk_events: Vec<(chrono::DateTime<chrono::Utc>, u64, u64)>,
    /// When each compaction happened and the tokens it saved, for the
    /// savings-over-time chart.
    #[serde(skip)]
    pub compaction_events: Vec<(chrono::DateTime<chrono::Utc>, u64)>,
    /// Estimated tokens pi-context-diet kept out of requests.
    #[serde(default)]
    pub diet_tokens: u64,
    /// Requests pi-context-diet trimmed.
    #[serde(default)]
    pub diet_requests: u64,
    /// pi-context-diet's savings per minute: `(minute, tokens, requests)`.
    #[serde(skip)]
    pub diet_events: Vec<(chrono::DateTime<chrono::Utc>, u64, u64)>,
    /// Estimated tokens pi-squeeze cut from bash output.
    #[serde(default)]
    pub squeeze_tokens: u64,
    /// Bash outputs pi-squeeze shortened.
    #[serde(default)]
    pub squeeze_outputs: u64,
    /// pi-squeeze's savings per minute: `(minute, tokens, outputs)`.
    #[serde(skip)]
    pub squeeze_events: Vec<(chrono::DateTime<chrono::Utc>, u64, u64)>,
}

impl ExternalSavings {
    /// The part that happened at a moment `keep` accepts, totals recomputed —
    /// how a selected hour, day or minute narrows the external sources.
    pub fn within(&self, keep: impl Fn(chrono::DateTime<chrono::Utc>) -> bool) -> ExternalSavings {
        let rtk_events: Vec<_> = self
            .rtk_events
            .iter()
            .filter(|(ts, _, _)| keep(*ts))
            .copied()
            .collect();
        let compaction_events: Vec<_> = self
            .compaction_events
            .iter()
            .filter(|(ts, _)| keep(*ts))
            .copied()
            .collect();
        let diet_events: Vec<_> = self
            .diet_events
            .iter()
            .filter(|(ts, _, _)| keep(*ts))
            .copied()
            .collect();
        let squeeze_events: Vec<_> = self
            .squeeze_events
            .iter()
            .filter(|(ts, _, _)| keep(*ts))
            .copied()
            .collect();
        ExternalSavings {
            rtk_tokens: rtk_events.iter().map(|e| e.1).sum(),
            rtk_commands: rtk_events.iter().map(|e| e.2).sum(),
            compaction_tokens: compaction_events.iter().map(|e| e.1).sum(),
            compactions: compaction_events.len() as u64,
            diet_tokens: diet_events.iter().map(|e| e.1).sum(),
            diet_requests: diet_events.iter().map(|e| e.2).sum(),
            squeeze_tokens: squeeze_events.iter().map(|e| e.1).sum(),
            squeeze_outputs: squeeze_events.iter().map(|e| e.2).sum(),
            rtk_events,
            compaction_events,
            diet_events,
            squeeze_events,
        }
    }
}

/// One source's contribution.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SourceSavings {
    pub source: SavingsSource,
    /// Tokens saved.
    pub tokens: u64,
    /// Estimated money saved, US dollars.
    pub usd: f64,
    /// Turns / commands / compactions behind the number (see [`SavingsSource::event_noun`]).
    pub events: u64,
}

/// Everything saved in the current view, per source and in total.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Savings {
    /// Sources that saved something, in [`SavingsSource::ALL`] order.
    pub sources: Vec<SourceSavings>,
    pub tokens: u64,
    /// Estimated money saved, US dollars.
    pub usd: f64,
}

impl Savings {
    /// Combine per-source numbers (zero ones are dropped).
    pub fn from_sources(sources: impl IntoIterator<Item = SourceSavings>) -> Self {
        let sources: Vec<SourceSavings> = sources.into_iter().filter(|s| s.tokens > 0).collect();
        Savings {
            tokens: sources.iter().map(|s| s.tokens).sum(),
            usd: sources.iter().map(|s| s.usd).sum(),
            sources,
        }
    }

    /// Build from the usage rows' cache reads plus the external sources, pricing
    /// tokens at `input_price_per_mtok` US dollars per million.
    pub fn build<'a>(
        rows: impl IntoIterator<Item = &'a crate::Row>,
        external: &ExternalSavings,
        input_price_per_mtok: f64,
    ) -> Self {
        let cache = super::ops::cache_savings(rows, input_price_per_mtok);
        Self::with_cache(cache, external, input_price_per_mtok)
    }

    /// Like [`Savings::build`] for callers that already summed the cache reads:
    /// `cache_tokens` read over `cache_turns` turns.
    pub fn from_totals(
        cache_tokens: u64,
        cache_turns: u64,
        external: &ExternalSavings,
        input_price_per_mtok: f64,
    ) -> Self {
        let cache = super::ops::cache_savings_of(cache_tokens, cache_turns, input_price_per_mtok);
        Self::with_cache(cache, external, input_price_per_mtok)
    }

    fn with_cache(cache: SourceSavings, external: &ExternalSavings, input_price: f64) -> Self {
        let price = sane_price(input_price);
        let rtk = SourceSavings {
            source: SavingsSource::Rtk,
            tokens: external.rtk_tokens,
            usd: usd_for(external.rtk_tokens, price),
            events: external.rtk_commands,
        };
        let compaction = SourceSavings {
            source: SavingsSource::PiCompaction,
            tokens: external.compaction_tokens,
            usd: usd_for(external.compaction_tokens, price),
            events: external.compactions,
        };
        let diet = SourceSavings {
            source: SavingsSource::PiContextDiet,
            tokens: external.diet_tokens,
            usd: diet_usd(external.diet_tokens, price),
            events: external.diet_requests,
        };
        // Like rtk: bash output cut before the model ever saw it, at the full input price.
        let squeeze = SourceSavings {
            source: SavingsSource::PiSqueeze,
            tokens: external.squeeze_tokens,
            usd: usd_for(external.squeeze_tokens, price),
            events: external.squeeze_outputs,
        };
        Self::from_sources([cache, rtk, compaction, diet, squeeze])
    }

    /// `true` when nothing was saved.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// The contribution of `source` (zero when absent).
    pub fn of(&self, source: SavingsSource) -> Option<&SourceSavings> {
        self.sources.iter().find(|s| s.source == source)
    }

    /// Saved money as a share of what it would have cost without any saving
    /// (`saved / (spent + saved)`), `None` when both are zero.
    pub fn percent_of_would_be(&self, spent_usd: f64) -> Option<f64> {
        let would_be = spent_usd + self.usd;
        (would_be > 0.0).then(|| self.usd / would_be)
    }
}

/// A usable price: finite and positive, otherwise the default.
pub(crate) fn sane_price(p: f64) -> f64 {
    if p.is_finite() && p > 0.0 {
        p
    } else {
        DEFAULT_INPUT_PRICE_PER_MTOK
    }
}

pub(crate) fn usd_for(tokens: u64, price_per_mtok: f64) -> f64 {
    tokens as f64 * price_per_mtok / 1e6
}

/// What pi-context-diet's elided tokens would have cost. Between cold starts they
/// would mostly have been prompt-cache reads, so they are priced as those
/// ([`CACHE_READ_PRICE_RATIO`] of the input price): a deliberately low estimate
/// that cannot double the prompt-cache saving.
pub(crate) fn diet_usd(tokens: u64, price_per_mtok: f64) -> f64 {
    usd_for(tokens, price_per_mtok) * CACHE_READ_PRICE_RATIO
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(source: SavingsSource, tokens: u64, usd: f64) -> SourceSavings {
        SourceSavings {
            source,
            tokens,
            usd,
            events: 1,
        }
    }

    #[test]
    fn totals_add_up_and_empty_sources_vanish() {
        let sv = Savings::from_sources([
            s(SavingsSource::PromptCache, 1000, 1.5),
            s(SavingsSource::Rtk, 0, 0.0),
            s(SavingsSource::PiCompaction, 500, 0.5),
        ]);
        assert_eq!((sv.tokens, sv.usd, sv.sources.len()), (1500, 2.0, 2));
        assert!(sv.of(SavingsSource::Rtk).is_none());
        assert_eq!(sv.of(SavingsSource::PiCompaction).unwrap().tokens, 500);
        assert!(!sv.is_empty());
        assert!(Savings::default().is_empty());
    }

    #[test]
    fn percent_of_would_be() {
        let sv = Savings::from_sources([s(SavingsSource::Rtk, 10, 3.0)]);
        assert_eq!(sv.percent_of_would_be(9.0), Some(0.25));
        assert_eq!(Savings::default().percent_of_would_be(0.0), None);
        assert_eq!(Savings::default().percent_of_would_be(5.0), Some(0.0));
    }

    #[test]
    fn bad_prices_fall_back_to_the_default() {
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(sane_price(bad), DEFAULT_INPUT_PRICE_PER_MTOK);
        }
        assert_eq!(sane_price(15.0), 15.0);
        assert_eq!(usd_for(2_000_000, 3.0), 6.0);
    }

    #[test]
    fn source_names_are_stable() {
        let names: Vec<_> = SavingsSource::ALL.iter().map(|s| s.as_str()).collect();
        assert_eq!(
            names,
            [
                "prompt_cache",
                "rtk",
                "pi_compaction",
                "pi_context_diet",
                "pi_squeeze"
            ]
        );
        assert_eq!(
            serde_json::to_string(&SavingsSource::PiSqueeze).unwrap(),
            "\"pi_squeeze\""
        );
        assert_eq!(
            (
                SavingsSource::PiSqueeze.label(),
                SavingsSource::PiSqueeze.event_noun()
            ),
            ("pi squeeze", "outputs")
        );
        assert_eq!(
            serde_json::to_string(&SavingsSource::PiContextDiet).unwrap(),
            "\"pi_context_diet\""
        );
        assert_eq!(
            serde_json::to_string(&SavingsSource::PiCompaction).unwrap(),
            "\"pi_compaction\""
        );
        for src in SavingsSource::ALL {
            assert!(!src.label().is_empty() && !src.event_noun().is_empty());
        }
    }
}

/// What was saved in one chart bucket (a minute, hour, day or month).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BucketSavings {
    /// The bucket label, the same string the chart and report use.
    pub label: String,
    pub savings: Savings,
}
