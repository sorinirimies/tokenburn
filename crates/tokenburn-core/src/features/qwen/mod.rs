//! Qwen Code collector (`~/.qwen/tmp/<project>/chats/*.json[l]`) — a Gemini CLI fork
//! that records chats in the same format.

pub mod ops;

pub use ops::{collect_qwen, collect_qwen_from, roots};
