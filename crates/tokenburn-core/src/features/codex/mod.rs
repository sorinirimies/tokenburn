//! OpenAI Codex CLI collector (`~/.codex/sessions/**/*.jsonl`).

pub mod ops;

pub use ops::{collect_codex, collect_codex_from, roots};
