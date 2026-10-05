//! OpenCode collector (`~/.local/share/opencode/opencode.db` + legacy JSON).

pub mod ops;

pub use ops::{collect_opencode, collect_opencode_from, roots};
