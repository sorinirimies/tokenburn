//! Colour themes shared by every front-end.

pub mod presets;
pub mod system;
pub mod types;

pub use presets::*;
pub use system::{
    current_system_theme_index, detect_system_dark, is_system_name, system_theme_index,
    ThemeChoice, SYSTEM_THEME_NAME,
};
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_name_has_a_theme_and_the_counts_agree() {
        assert_eq!(THEME_NAMES.len(), THEME_COUNT);
        for i in 0..THEME_COUNT {
            let t = theme_by_index(i);
            assert_ne!(t.background, t.text_primary, "theme {i} is unreadable");
            assert_ne!(
                t.border, t.accent,
                "theme {i}: border and accent are identical"
            );
        }
    }

    #[test]
    fn names_are_unique_and_found_case_insensitively() {
        for (i, n) in THEME_NAMES.iter().enumerate() {
            assert_eq!(
                THEME_NAMES.iter().filter(|m| m == &n).count(),
                1,
                "duplicate {n}"
            );
            assert_eq!(find_theme(&n.to_ascii_uppercase()), Some(i));
        }
        assert_eq!(find_theme("  nord  "), find_theme("Nord"));
        assert_eq!(find_theme("no such theme"), None);
        assert_eq!(theme_index_by_name("no such theme"), 0);
    }

    #[test]
    fn dark_and_light_themes_have_matching_luminance() {
        // `is_dark` must agree with the actual background so front-ends can rely on it
        // (CSS `color-scheme`, iced palette).
        let lum = |c: Rgb| 0.299 * f32::from(c.r) + 0.587 * f32::from(c.g) + 0.114 * f32::from(c.b);
        for (i, name) in THEME_NAMES.iter().enumerate() {
            let t = theme_by_index(i);
            assert_eq!(
                t.is_dark,
                lum(t.background) < 128.0,
                "{name} says is_dark={}",
                t.is_dark
            );
        }
    }

    #[test]
    fn there_are_light_and_dark_themes() {
        let dark = (0..THEME_COUNT)
            .filter(|i| theme_by_index(*i).is_dark)
            .count();
        assert!(dark > 10 && dark < THEME_COUNT - 5);
    }

    #[test]
    fn text_contrasts_with_the_background_in_every_theme() {
        // WCAG-ish: primary text must be clearly distinguishable from the background.
        let lum = |c: Rgb| 0.299 * f32::from(c.r) + 0.587 * f32::from(c.g) + 0.114 * f32::from(c.b);
        for (i, name) in THEME_NAMES.iter().enumerate() {
            let t = theme_by_index(i);
            let d = (lum(t.text_primary) - lum(t.background)).abs();
            assert!(
                d > 90.0,
                "{name}: text/background luminance gap only {d:.0}"
            );
        }
    }

    #[test]
    fn hex_and_unit_conversions() {
        assert_eq!(Rgb::new(255, 0, 16).hex(), "#ff0010");
        assert_eq!(Rgb::new(255, 0, 0).unit(), (1.0, 0.0, 0.0));
    }

    #[test]
    fn every_tool_gets_a_series_colour() {
        let t = theme_by_index(0);
        for tool in crate::Tool::ALL {
            let _ = t.tool_color(tool);
        }
        // Eight series colours: the first eight tools are all distinct, later ones wrap.
        let idx: std::collections::HashSet<_> = crate::Tool::ALL
            .iter()
            .take(8)
            .map(|t| t.series_index() % 8)
            .collect();
        assert_eq!(idx.len(), 8);
    }
}
