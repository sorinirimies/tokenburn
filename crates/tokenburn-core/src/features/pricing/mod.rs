//! Model prices: what a million tokens of a model cost, to put a dollar figure on
//! usage whose log has none (Zed's agent thread database records tokens only).
//!
//! The table is data, not code: [`prices.json`](https://github.com/sorinirimies/tokenburn/blob/main/crates/tokenburn-core/src/features/pricing/prices.json)
//! is embedded, described by `prices.schema.json`, and a `prices.json` in the
//! TokenBurn config directory adds to it or replaces entries. Costs the tools log
//! themselves are never touched; a model without a price keeps cost `0`.

pub mod ops;
pub mod types;

pub use ops::{normalize, prices_path};
pub use types::{Price, Prices};
