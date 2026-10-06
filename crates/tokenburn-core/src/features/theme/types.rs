//! Platform-agnostic theme types.
//!
//! The shared colour definitions that every front-end converts from (ratatui
//! `Color::Rgb`, iced `Color`, CSS custom properties). Defined once in core so a
//! theme name renders the same palette in the TUI, the GUI and the web dashboard.

/// A single RGB colour (0–255 per channel).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    /// Usable in `const` contexts.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// `#rrggbb`.
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// `rgb / 255` as floats (iced wants `0.0..=1.0`).
    pub fn unit(self) -> (f32, f32, f32) {
        (
            f32::from(self.r) / 255.0,
            f32::from(self.g) / 255.0,
            f32::from(self.b) / 255.0,
        )
    }
}

/// A complete UI colour theme — every semantic slot the front-ends need.
#[derive(Debug, Clone)]
pub struct AppTheme {
    /// `true` for dark themes, `false` for light themes.
    pub is_dark: bool,

    // ── Structural colours ───────────────────────────────────────────────
    /// Main window / terminal / page background.
    pub background: Rgb,
    /// Slightly elevated surface (panels, cards, tables).
    pub surface: Rgb,
    /// Borders and dividers.
    pub border: Rgb,
    /// Background for selected / highlighted rows.
    pub selection: Rgb,

    // ── Text colours ─────────────────────────────────────────────────────
    pub text_primary: Rgb,
    pub text_secondary: Rgb,
    pub text_muted: Rgb,

    // ── Semantic colours ─────────────────────────────────────────────────
    /// Accent / primary action colour.
    pub accent: Rgb,
    /// Success — also used for cost figures.
    pub success: Rgb,
    pub warning: Rgb,
    pub error: Rgb,

    /// Eight series colours: one per tool in charts, pies and bars (see
    /// [`crate::Tool::series_index`]). Every theme picks eight that stay legible
    /// on its own background.
    pub series: [Rgb; 8],
}

impl AppTheme {
    /// The series colour for a tool.
    pub fn tool_color(&self, tool: crate::Tool) -> Rgb {
        self.series[tool.series_index() % self.series.len()]
    }
}
