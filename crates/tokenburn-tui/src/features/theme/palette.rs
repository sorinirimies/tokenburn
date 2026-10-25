use ratatui::style::Color;
use std::sync::OnceLock;
use tokenburn_core::Tool;
use tui_nuances::Theme;

/// Every colour slot the UI needs, as ratatui colours. Views read these instead
/// of hard-coding `Color::*`, so every theme restyles the whole screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiTheme {
    /// Screen background.
    pub bg: Color,
    /// Elevated surface (status bar, popups).
    pub surface: Color,
    /// Inactive borders and dividers.
    pub border: Color,
    /// Selected-row background.
    pub selection: Color,
    pub text: Color,
    pub text_secondary: Color,
    pub muted: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    /// One colour per tool / pie slice.
    pub series: [Color; 8],
}

impl UiTheme {
    /// Derive the UI colours from a [`tui_nuances::Theme`], the source of truth for every palette.
    pub fn from_nuances(t: &Theme) -> Self {
        let bg = t.background.unwrap_or(Color::Reset);
        Self {
            bg,
            surface: t.surface.unwrap_or(bg),
            border: t.border,
            selection: t.selection_bg,
            text: t.foreground,
            text_secondary: t.secondary,
            muted: t.muted,
            accent: t.accent,
            success: t.success,
            warning: t.warning,
            error: t.error,
            series: std::array::from_fn(|i| t.color(i)),
        }
    }

    /// Theme by index into [`tokenburn_core::THEME_NAMES`] (out of range → Default).
    /// Colours come from the `tui-nuances` theme of the same name.
    pub fn by_index(index: usize) -> Self {
        static CACHE: OnceLock<Vec<UiTheme>> = OnceLock::new();
        let all = CACHE.get_or_init(|| {
            tokenburn_core::THEME_NAMES
                .iter()
                .map(|n| {
                    let t = Theme::by_name(n).unwrap_or_default();
                    Self::from_nuances(&t)
                })
                .collect()
        });
        all.get(index).copied().unwrap_or(all[0])
    }

    /// The colour of a tool's bars / slices.
    pub fn tool(&self, name: &str) -> Color {
        Tool::ALL
            .iter()
            .position(|t| t.as_str() == name)
            .map_or(self.text, |i| self.series[i % self.series.len()])
    }

    /// The `i`-th series colour, wrapping.
    pub fn slice(&self, i: usize) -> Color {
        self.series[i % self.series.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_resolves_to_truecolor() {
        for i in 0..tokenburn_core::THEME_COUNT {
            let t = UiTheme::by_index(i);
            assert!(matches!(t.accent, Color::Rgb(..)) || i == 0 || t.bg == Color::Reset);
            assert_ne!(t.bg, t.text, "theme {i}");
        }
    }

    #[test]
    fn out_of_range_falls_back_to_default() {
        assert_eq!(UiTheme::by_index(9999), UiTheme::by_index(0));
    }

    #[test]
    fn tool_colours_are_distinct_and_unknown_tools_use_the_text_colour() {
        let t = UiTheme::by_index(0);
        let colors: Vec<Color> = Tool::ALL
            .iter()
            .take(8) // eight series colours; later tools wrap around
            .map(|x| t.tool(x.as_str()))
            .collect();
        for (i, c) in colors.iter().enumerate() {
            assert!(!colors[..i].contains(c), "tool {i} shares a colour");
        }
        assert_eq!(t.tool("mystery"), t.text);
    }

    #[test]
    fn colours_come_from_tui_nuances() {
        let n = Theme::by_name("Dracula").unwrap();
        let i = tokenburn_core::theme_index_by_name("Dracula");
        assert_eq!(UiTheme::by_index(i).accent, n.accent);
    }
}
