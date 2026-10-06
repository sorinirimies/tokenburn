//! Colour themes: every theme from `tokenburn-core`, shown in a pick list.

use std::fmt;

use iced::theme::Palette;
use iced::{Color, Theme};
use tokenburn_core::{theme_by_index, Rgb, THEME_NAMES};

/// One entry of the theme pick list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeChoice {
    pub index: usize,
    pub name: &'static str,
}

impl ThemeChoice {
    /// The choice for `index` (clamped into range).
    pub fn at(index: usize) -> Self {
        let index = index.min(THEME_NAMES.len() - 1);
        Self {
            index,
            name: THEME_NAMES[index],
        }
    }

    /// Every theme, in the canonical order.
    pub fn all() -> Vec<ThemeChoice> {
        (0..THEME_NAMES.len()).map(Self::at).collect()
    }
}

impl fmt::Display for ThemeChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

pub fn rgb_to_color(c: Rgb) -> Color {
    let (r, g, b) = c.unit();
    Color::from_rgb(r, g, b)
}

/// The colours the canvas charts need, resolved from theme `index`.
pub fn chart_colors(index: usize) -> crate::widgets::charts::ChartColors {
    let t = theme_by_index(index);
    crate::widgets::charts::ChartColors {
        series: t.series.map(rgb_to_color),
        text: rgb_to_color(t.text_primary),
        muted: rgb_to_color(t.text_muted),
        grid: rgb_to_color(t.border),
        background: rgb_to_color(t.surface),
        accent: rgb_to_color(t.accent),
    }
}

/// A **custom** iced theme built from the core theme at `index`.
///
/// Deriving the `Palette` is what makes every built-in widget (buttons, pick
/// lists, scrollables, progress bars, `text::primary` …) pick up the theme,
/// instead of iced falling back to its generic dark/light palette.
pub fn iced_theme(index: usize) -> Theme {
    let t = theme_by_index(index);
    let name = THEME_NAMES[index.min(THEME_NAMES.len() - 1)];
    Theme::custom(
        name.to_string(),
        Palette {
            background: rgb_to_color(t.background),
            text: rgb_to_color(t.text_primary),
            primary: rgb_to_color(t.accent),
            success: rgb_to_color(t.success),
            warning: rgb_to_color(t.warning),
            danger: rgb_to_color(t.error),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_becomes_a_named_custom_iced_theme() {
        for (i, name) in THEME_NAMES.iter().enumerate() {
            let t = iced_theme(i);
            assert_eq!(t.to_string(), *name);
            let (r, g, b) = theme_by_index(i).accent.unit();
            assert_eq!(t.palette().primary, Color::from_rgb(r, g, b), "{name}");
        }
    }

    #[test]
    fn choices_cover_every_theme_once_and_clamp() {
        let all = ThemeChoice::all();
        assert_eq!(all.len(), tokenburn_core::THEME_COUNT);
        assert_eq!(ThemeChoice::at(9999).index, all.len() - 1);
        assert_eq!(all[0].to_string(), "Default");
    }

    #[test]
    fn out_of_range_index_does_not_panic() {
        let _ = iced_theme(usize::MAX);
        let _ = chart_colors(usize::MAX);
    }

    #[test]
    fn chart_colours_follow_the_theme() {
        let (a, b) = (chart_colors(0), chart_colors(9));
        assert_ne!(a.background, b.background);
        assert_ne!(a.series, b.series);
        assert_eq!(a.series.len(), 8);
    }
}
