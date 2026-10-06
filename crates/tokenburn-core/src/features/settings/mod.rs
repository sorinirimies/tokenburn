//! User settings shared by every front-end (`~/.config/tokenburn/settings.json`).
//!
//! One file, so the theme you pick in the TUI is the one the GUI and the web
//! dashboard start with.

pub mod ops;
pub mod types;

pub use ops::{config_dir, load, load_from, save, save_theme, save_to, settings_path, theme_index};
pub use types::Settings;
