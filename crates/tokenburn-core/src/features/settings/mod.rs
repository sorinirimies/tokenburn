//! User settings shared by every front-end (`~/.config/tokenburn/settings.json`).
//!
//! One file, so the theme you pick in the TUI is the one the GUI and the web
//! dashboard start with.

pub mod ops;
pub mod types;

pub use ops::{
    config_dir, currency, follow_system_theme, input_price, load, load_from, save, save_currency,
    save_system_theme, save_theme, save_to, settings_path, theme_index,
};
pub use types::Settings;
