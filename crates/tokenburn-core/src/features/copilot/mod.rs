//! GitHub Copilot CLI collector (`~/.copilot/session-state`).

pub mod ops;

pub use ops::{collect_copilot, collect_copilot_from, root};
