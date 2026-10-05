//! Google Gemini CLI collector (`~/.gemini/tmp/*/chats`).

pub mod ops;

pub use ops::{collect_gemini, collect_gemini_from, roots};
