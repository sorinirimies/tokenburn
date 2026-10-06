//! The options popup: checkboxes ([`tui_checkbox`]) and sliders ([`tui_slider`]).

pub mod events;
pub mod view;

use std::time::Duration;

use tui_slider::SliderState;

/// One selectable row of the options popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionRow {
    AutoRefresh,
    RefreshSecs,
    ShowChart,
    ShowPie,
    HiResPie,
    TopN,
}

impl OptionRow {
    pub const ALL: [OptionRow; 6] = [
        OptionRow::AutoRefresh,
        OptionRow::RefreshSecs,
        OptionRow::ShowChart,
        OptionRow::ShowPie,
        OptionRow::HiResPie,
        OptionRow::TopN,
    ];

    pub fn is_slider(self) -> bool {
        matches!(self, OptionRow::RefreshSecs | OptionRow::TopN)
    }
}

/// User-adjustable TUI settings.
#[derive(Debug, Clone)]
pub struct Options {
    /// Popup visible?
    pub open: bool,
    pub cursor: OptionRow,
    /// Re-read the logs every [`Options::refresh_interval`].
    pub auto_refresh: bool,
    pub show_chart: bool,
    pub show_pie: bool,
    /// Braille (8× resolution) pie chart.
    pub hires_pie: bool,
    /// Auto-refresh period in seconds (5–300, step 5).
    pub refresh_secs: SliderState,
    /// Pie slices before the rest is folded into "other" (3–12).
    pub top_n: SliderState,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            open: false,
            cursor: OptionRow::AutoRefresh,
            auto_refresh: true,
            show_chart: true,
            show_pie: true,
            hires_pie: true,
            refresh_secs: SliderState::with_step(5.0, 5.0, 300.0, 5.0),
            top_n: SliderState::with_step(6.0, 3.0, 12.0, 1.0),
        }
    }
}

impl Options {
    pub fn refresh_interval(&self) -> Duration {
        Duration::from_secs(self.refresh_secs.value().round().max(1.0) as u64)
    }

    pub fn top_n(&self) -> usize {
        self.top_n.value().round().max(1.0) as usize
    }

    /// Move the cursor by `delta` rows, wrapping.
    pub fn move_cursor(&mut self, delta: isize) {
        let n = OptionRow::ALL.len() as isize;
        let i = OptionRow::ALL
            .iter()
            .position(|r| *r == self.cursor)
            .unwrap_or(0) as isize;
        self.cursor = OptionRow::ALL[((i + delta).rem_euclid(n)) as usize];
    }

    /// Space/Enter: flip the selected checkbox (no-op on sliders).
    pub fn toggle(&mut self) {
        match self.cursor {
            OptionRow::AutoRefresh => self.auto_refresh = !self.auto_refresh,
            OptionRow::ShowChart => self.show_chart = !self.show_chart,
            OptionRow::ShowPie => self.show_pie = !self.show_pie,
            OptionRow::HiResPie => self.hires_pie = !self.hires_pie,
            OptionRow::RefreshSecs | OptionRow::TopN => {}
        }
    }

    /// Left/Right: step the selected slider (`+1`/`-1`), or flip a checkbox.
    pub fn adjust(&mut self, dir: i32) {
        match self.cursor {
            OptionRow::RefreshSecs => step(&mut self.refresh_secs, dir),
            OptionRow::TopN => step(&mut self.top_n, dir),
            _ => self.toggle(),
        }
    }
}

fn step(s: &mut SliderState, dir: i32) {
    if dir >= 0 {
        s.step_up();
    } else {
        s.step_down();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let o = Options::default();
        assert!(o.auto_refresh && o.show_chart && o.show_pie);
        assert_eq!(o.refresh_interval(), Duration::from_secs(5));
        assert_eq!(o.top_n(), 6);
    }

    #[test]
    fn cursor_wraps_both_ways() {
        let mut o = Options::default();
        o.move_cursor(-1);
        assert_eq!(o.cursor, OptionRow::TopN);
        o.move_cursor(1);
        assert_eq!(o.cursor, OptionRow::AutoRefresh);
    }

    #[test]
    fn toggle_flips_checkboxes_and_ignores_sliders() {
        let mut o = Options::default();
        o.toggle();
        assert!(!o.auto_refresh);
        o.cursor = OptionRow::TopN;
        let before = o.top_n();
        o.toggle();
        assert_eq!(o.top_n(), before);
    }

    #[test]
    fn adjust_steps_sliders_within_bounds() {
        let mut o = Options {
            cursor: OptionRow::RefreshSecs,
            ..Options::default()
        };
        o.adjust(1);
        assert_eq!(o.refresh_interval(), Duration::from_secs(10));
        for _ in 0..100 {
            o.adjust(-1);
        }
        assert_eq!(
            o.refresh_interval(),
            Duration::from_secs(5),
            "clamped at min"
        );
        o.cursor = OptionRow::TopN;
        for _ in 0..100 {
            o.adjust(1);
        }
        assert_eq!(o.top_n(), 12, "clamped at max");
    }

    #[test]
    fn adjust_on_a_checkbox_toggles_it() {
        let mut o = Options {
            cursor: OptionRow::ShowPie,
            ..Options::default()
        };
        o.adjust(1);
        assert!(!o.show_pie);
    }
}
