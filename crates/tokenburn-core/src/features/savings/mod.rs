//! What you *saved*, next to what you spent.
//!
//! Three sources, all read from local data (no network):
//!
//! * **Prompt cache** — tokens the provider served from its cache
//!   (`cache_read`) instead of billing at the full input price. Every tool that
//!   logs cache reads contributes.
//! * **rtk** — [rtk](https://github.com/rtk-ai/rtk) filters command output
//!   before it reaches the model and records the tokens it dropped in its own
//!   `history.db` (see [`rtk`]).
//! * **pi compaction** — pi summarises old context; each compaction entry in a
//!   session log records the context size before it ([`crate::features::pi`]).
//!
//! The money figure prices every saved token at the user's input price
//! ([`Settings::input_price_per_mtok`](crate::Settings)); prompt-cache hits are
//! only discounted, not free, so they are credited with
//! `1 - `[`CACHE_READ_PRICE_RATIO`] of that price. All of it is an *estimate*.

pub mod ops;
pub mod rtk;
pub mod types;

pub use ops::{avoided_usd, cache_savings, cache_usd, collect_external, savings_by_bucket};
pub use types::{
    BucketSavings, ExternalSavings, Savings, SavingsSource, SourceSavings, CACHE_READ_PRICE_RATIO,
    DEFAULT_INPUT_PRICE_PER_MTOK,
};
