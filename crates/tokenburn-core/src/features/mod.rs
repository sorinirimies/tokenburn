//! Feature modules — one per concern, mirroring the front-end crates.
//!
//! Collectors (one per supported tool): `pi`, `zed`, `claude`, `codex`,
//! `copilot`, `gemini`, `opencode`, `amp`, `cline` (+ Roo/Kilo), `qwen`. Aggregation: `report`. Shared data
//! types: `usage`.

pub mod amp;
pub mod claude;
pub mod cline;
pub mod codex;
pub mod copilot;
pub mod currency;
pub mod gemini;
pub mod limits;
pub mod opencode;
pub mod pi;
pub mod qwen;
pub mod report;
pub mod savings;
pub mod settings;
pub mod theme;
pub mod usage;
pub mod zed;
