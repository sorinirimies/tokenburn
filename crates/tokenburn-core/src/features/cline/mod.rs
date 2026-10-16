//! Cline, Roo Code and Kilo Code collectors. All three are VS Code extensions (Cline
//! forks) that keep `tasks/<id>/ui_messages.json` under the editor's `globalStorage`.

pub mod ops;

pub use ops::{collect_cline, collect_cline_from, collect_kilo, collect_roo, roots};
