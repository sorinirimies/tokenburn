//! Feature modules — one per concern, mirroring the front-end crates.
//!
//! Collectors (one per supported tool): `pi`, `zed`, `claude`, `codex`,
//! `copilot`, `gemini`, `opencode`, `amp`. Aggregation: `report`. Shared data
//! types: `usage`.

pub mod amp;
pub mod claude;
pub mod codex;
pub mod copilot;
pub mod gemini;
pub mod opencode;
pub mod pi;
pub mod report;
pub mod settings;
pub mod theme;
pub mod usage;
pub mod zed;
