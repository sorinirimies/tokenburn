//! Responsive breakpoints shared by every front-end, so a "narrow" window means the
//! same thing in the TUI, the GUI and the web dashboard:
//!
//! | class | window | terminal | what changes |
//! |---|---|---|---|
//! | **Wide** | ≥ 980 px | ≥ 110 columns | everything: all columns, charts side by side |
//! | **Medium** | ≥ 700 px | ≥ 70 columns | the cache columns drop out, charts stack |
//! | **Narrow** | below | below | compact header and controls; only the essential columns |
//!
//! The pixel and column thresholds are the same classes in different units; the
//! front-ends decide what each class looks like.

/// Window width classes, narrowest first (so `class >= LayoutClass::Medium` reads
/// naturally).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LayoutClass {
    Narrow,
    Medium,
    Wide,
}

/// Minimum logical-pixel width of a wide / medium window (GUI and web).
pub const WIDE_PX: f32 = 980.0;
pub const MEDIUM_PX: f32 = 700.0;
/// Minimum width in terminal columns of a wide / medium terminal (TUI).
pub const WIDE_COLS: u16 = 110;
pub const MEDIUM_COLS: u16 = 70;

impl LayoutClass {
    /// Class of a window `width` logical pixels wide.
    pub fn from_px(width: f32) -> Self {
        if width >= WIDE_PX {
            Self::Wide
        } else if width >= MEDIUM_PX {
            Self::Medium
        } else {
            Self::Narrow
        }
    }

    /// Class of a terminal `cols` columns wide.
    pub fn from_cols(cols: u16) -> Self {
        if cols >= WIDE_COLS {
            Self::Wide
        } else if cols >= MEDIUM_COLS {
            Self::Medium
        } else {
            Self::Narrow
        }
    }

    pub fn is_narrow(self) -> bool {
        self == Self::Narrow
    }

    pub fn is_wide(self) -> bool {
        self == Self::Wide
    }

    /// `"wide"`, `"medium"` or `"narrow"`.
    pub fn label(self) -> &'static str {
        match self {
            Self::Wide => "wide",
            Self::Medium => "medium",
            Self::Narrow => "narrow",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_widths_fall_into_the_documented_classes() {
        assert_eq!(LayoutClass::from_px(1920.0), LayoutClass::Wide);
        assert_eq!(
            LayoutClass::from_px(980.0),
            LayoutClass::Wide,
            "the boundary is wide"
        );
        assert_eq!(LayoutClass::from_px(979.9), LayoutClass::Medium);
        assert_eq!(LayoutClass::from_px(700.0), LayoutClass::Medium);
        assert_eq!(LayoutClass::from_px(699.9), LayoutClass::Narrow);
        assert_eq!(LayoutClass::from_px(0.0), LayoutClass::Narrow);
        assert_eq!(
            LayoutClass::from_px(f32::NAN),
            LayoutClass::Narrow,
            "garbage is narrow"
        );
    }

    #[test]
    fn terminal_columns_fall_into_the_same_classes() {
        assert_eq!(LayoutClass::from_cols(200), LayoutClass::Wide);
        assert_eq!(LayoutClass::from_cols(110), LayoutClass::Wide);
        assert_eq!(LayoutClass::from_cols(109), LayoutClass::Medium);
        assert_eq!(LayoutClass::from_cols(70), LayoutClass::Medium);
        assert_eq!(LayoutClass::from_cols(69), LayoutClass::Narrow);
        assert_eq!(LayoutClass::from_cols(0), LayoutClass::Narrow);
        assert_eq!(
            LayoutClass::from_cols(80),
            LayoutClass::Medium,
            "the classic 80x24"
        );
    }

    #[test]
    fn classes_order_from_narrow_to_wide() {
        assert!(
            LayoutClass::Narrow < LayoutClass::Medium && LayoutClass::Medium < LayoutClass::Wide
        );
        assert!(LayoutClass::Narrow.is_narrow() && LayoutClass::Wide.is_wide());
        assert_eq!(LayoutClass::Medium.label(), "medium");
    }
}
