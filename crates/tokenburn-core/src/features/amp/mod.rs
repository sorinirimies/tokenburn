//! Amp collector (`~/.local/share/amp/threads/T-*.json`).

pub mod ops;

pub use ops::{collect_amp, collect_amp_from, roots};
