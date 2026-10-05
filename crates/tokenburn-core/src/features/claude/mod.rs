//! Claude Code collector (`~/.claude/projects/**/*.jsonl`).

pub mod ops;

pub use ops::{collect_claude, collect_claude_from, roots};
