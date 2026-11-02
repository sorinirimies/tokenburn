//! What you *saved*, next to what you spent.
//!
//! Four sources, all read from local data (no network):
//!
//! * **Prompt cache** — tokens the provider served from its cache
//!   (`cache_read`) instead of billing at the full input price. Every tool that
//!   logs cache reads contributes.
//! * **rtk** — [rtk](https://github.com/rtk-ai/rtk) filters command output
//!   before it reaches the model and records the tokens it dropped in its own
//!   `history.db` (see [`rtk`]).
//! * **pi compaction** — pi summarises old context; each compaction entry in a
//!   session log records the context size before it ([`crate::features::pi`]).
//! * **pi context-diet** — [pi-context-diet](https://github.com/sorinirimies/pi-context-diet)
//!   elides stale tool results and logs the tokens each request left out
//!   (see [`context_diet`]).
//!
//! The money figure prices every saved token at the user's input price
//! ([`Settings::input_price_per_mtok`](crate::Settings)); prompt-cache hits are
//! only discounted, not free, so they are credited with
//! `1 - `[`CACHE_READ_PRICE_RATIO`] of that price, and context-diet's tokens
//! (mostly would-be cache reads) with [`CACHE_READ_PRICE_RATIO`] of it. All of
//! it is an *estimate*.

pub mod context_diet;
pub mod ops;
pub mod rtk;
pub mod types;

pub use ops::{
    avoided_usd, cache_savings, cache_usd, collect_external, context_diet_usd, savings_by_bucket,
};
pub use types::{
    BucketSavings, ExternalSavings, Savings, SavingsSource, SourceSavings, CACHE_READ_PRICE_RATIO,
    DEFAULT_INPUT_PRICE_PER_MTOK,
};
