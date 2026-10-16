//! Colour themes: every theme from `tokenburn-core`, a picker popup, persistence.

pub mod events;
pub mod palette;
pub mod view;

pub use palette::UiTheme;

/// Picker rows: "System" first, then every core preset.
pub const PICKER_LEN: usize = tokenburn_core::THEME_COUNT + 1;

/// Label of picker row `i` (`0` = System, `n + 1` = preset `n`).
pub fn picker_name(i: usize) -> &'static str {
    match i.checked_sub(1) {
        None => tokenburn_core::SYSTEM_THEME_NAME,
        Some(n) => tokenburn_core::THEME_NAMES[n.min(tokenburn_core::THEME_COUNT - 1)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_lists_system_first_then_every_preset() {
        assert_eq!(PICKER_LEN, tokenburn_core::THEME_COUNT + 1);
        assert_eq!(picker_name(0), "System");
        assert_eq!(picker_name(1), tokenburn_core::THEME_NAMES[0]);
        assert_eq!(
            picker_name(PICKER_LEN - 1),
            *tokenburn_core::THEME_NAMES.last().unwrap()
        );
    }
}
