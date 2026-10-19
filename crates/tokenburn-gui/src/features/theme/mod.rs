//! Colour themes: every theme from `tokenburn-core`, shown in a pick list.

use std::fmt;

use iced::theme::Palette;
use iced::{Color, Theme};
use tokenburn_core::{theme_by_index, Rgb, THEME_NAMES};

/// One entry of the theme pick list. `index == THEME_COUNT` is the "System"
/// (follow the OS light/dark setting) entry; anything below is a preset.
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

    /// The "follow system" entry.
    pub fn system() -> Self {
        Self {
            index: SYSTEM_INDEX,
            name: tokenburn_core::SYSTEM_THEME_NAME,
        }
    }

    /// Whether this is the "follow system" entry.
    pub fn is_system(&self) -> bool {
        self.index == SYSTEM_INDEX
    }

    /// The entry matching the app state: System while following, else the preset.
    pub fn selected(index: usize, follow_system: bool) -> Self {
        if follow_system {
            Self::system()
        } else {
            Self::at(index)
        }
    }

    /// "System" first, then every preset in the canonical order.
    pub fn all() -> Vec<ThemeChoice> {
        std::iter::once(Self::system())
            .chain((0..THEME_NAMES.len()).map(Self::at))
            .collect()
    }
}

/// Index of the "System" pick-list entry (one past the last preset).
pub const SYSTEM_INDEX: usize = tokenburn_core::THEME_COUNT;

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

/// Clearly outlined secondary button: accent-tinted fill and border, strong
/// hover/press feedback, dimmed when disabled.
pub fn action_button(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    use iced::widget::button::{Status, Style};
    let p = theme.palette();
    let base = p.background;
    let mix = |a: Color, b: Color, t: f32| Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: 1.0,
    };
    let surface = theme.extended_palette().background.weak.color;
    let (bg, border, text) = match status {
        Status::Active => (
            mix(surface, p.primary, 0.12),
            mix(surface, p.primary, 0.55),
            p.text,
        ),
        Status::Hovered => (mix(surface, p.primary, 0.28), p.primary, p.text),
        Status::Pressed => (mix(surface, p.primary, 0.42), p.primary, p.text),
        Status::Disabled => (
            mix(surface, base, 0.4),
            mix(surface, base, 0.4),
            mix(p.text, base, 0.6),
        ),
    };
    Style {
        background: Some(bg.into()),
        text_color: text,
        border: iced::Border {
            color: border,
            width: 1.0,
            radius: 7.0.into(),
        },
        ..Style::default()
    }
}

/// Solid accent button for the selected / primary state.
pub fn primary_button(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    use iced::widget::button::{Status, Style};
    let p = theme.palette();
    let lum = 0.299 * p.primary.r + 0.587 * p.primary.g + 0.114 * p.primary.b;
    let on = if lum > 0.6 {
        Color::from_rgb(0.06, 0.07, 0.1)
    } else {
        Color::WHITE
    };
    let mix = |a: Color, b: Color, t: f32| Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: 1.0,
    };
    let bg = match status {
        Status::Active => p.primary,
        Status::Hovered => mix(p.primary, Color::WHITE, 0.18),
        Status::Pressed => mix(p.primary, Color::BLACK, 0.2),
        Status::Disabled => mix(p.background, p.primary, 0.2),
    };
    Style {
        background: Some(bg.into()),
        text_color: on,
        border: iced::Border {
            color: mix(p.primary, Color::WHITE, 0.35),
            width: 1.0,
            radius: 7.0.into(),
        },
        ..Style::default()
    }
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
        assert_eq!(all.len(), tokenburn_core::THEME_COUNT + 1);
        assert_eq!(ThemeChoice::at(9999).index, all.len() - 2);
        assert_eq!(all[0].to_string(), "System");
        assert!(all[0].is_system() && !all[1].is_system());
        assert_eq!(all[1].to_string(), "Default");
    }

    #[test]
    fn selected_is_system_only_while_following() {
        assert!(ThemeChoice::selected(5, true).is_system());
        assert_eq!(ThemeChoice::selected(5, false), ThemeChoice::at(5));
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
