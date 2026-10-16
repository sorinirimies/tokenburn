use ratatui::style::Color;
use tokenburn_core::{AppTheme, Rgb, Tool};

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
    /// Convert from the core's platform-agnostic theme.
    pub fn from_core(t: &AppTheme) -> Self {
        Self {
            bg: rgb(t.background),
            surface: rgb(t.surface),
            border: rgb(t.border),
            selection: rgb(t.selection),
            text: rgb(t.text_primary),
            text_secondary: rgb(t.text_secondary),
            muted: rgb(t.text_muted),
            accent: rgb(t.accent),
            success: rgb(t.success),
            warning: rgb(t.warning),
            error: rgb(t.error),
            series: t.series.map(rgb),
        }
    }

    /// Theme by index into [`tokenburn_core::THEME_NAMES`] (out of range → Default).
    pub fn by_index(index: usize) -> Self {
        Self::from_core(&tokenburn_core::theme_by_index(index))
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

fn rgb(c: Rgb) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_resolves_to_truecolor() {
        for i in 0..tokenburn_core::THEME_COUNT {
            let t = UiTheme::by_index(i);
            assert!(matches!(t.bg, Color::Rgb(..)) && matches!(t.accent, Color::Rgb(..)));
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
    fn rgb_round_trips() {
        let core = tokenburn_core::theme_by_index(9);
        assert_eq!(
            UiTheme::from_core(&core).accent,
            Color::Rgb(core.accent.r, core.accent.g, core.accent.b)
        );
    }
}
