//! Charts drawn on an iced canvas: stacked bars over time and a share donut.
//!
//! Both are plain [`canvas::Program`]s over data that `tokenburn-core` builds
//! ([`TimeSeries`], [`Pie`]), coloured from the active theme.
//!
//! They are interactive: hover highlights (and a tooltip on the bars), a click
//! selects a bucket or a project (see `tokenburn_core::Selection`), and they
//! animate — the bars grow in with a stagger, the donut sweeps round, and a
//! selected slice pops out while the rest fade. The animation values come from
//! the app state (`anim`, `sel_anim`), driven by display frames.

use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke, Text};
use iced::{alignment, Color, Element, Length, Pixels, Point, Rectangle, Renderer, Size, Theme};
use std::f32::consts::{FRAC_PI_2, TAU};
use tokenburn_core::{Metric, Pie, TimeSeries, Tool};

use tokenburn_core::{axis_label, bucket_at, nice_max, short_label, slice_angles};

use crate::message::Message;

/// Colours a chart needs, resolved from the theme once.
#[derive(Debug, Clone, Copy)]
pub struct ChartColors {
    pub series: [Color; 8],
    pub text: Color,
    pub muted: Color,
    pub grid: Color,
    pub background: Color,
    pub accent: Color,
}

/// Points of a ring segment (a donut slice): the outer arc forward, then the
/// inner arc back. Straight edges every ~3° look like a true arc at chart sizes
/// and, unlike path arcs, fill reliably on every renderer.
pub fn ring_segment(center: Point, r_out: f32, r_in: f32, start: f32, end: f32) -> Vec<Point> {
    let steps = (((end - start).abs().to_degrees() / 3.0).ceil() as usize).max(1);
    let at = |r: f32, t: f32| Point::new(center.x + r * t.cos(), center.y + r * t.sin());
    let angle = |i: usize| start + (end - start) * i as f32 / steps as f32;
    (0..=steps)
        .map(|i| at(r_out, angle(i)))
        .chain((0..=steps).rev().map(|i| at(r_in, angle(i))))
        .collect()
}

// ── shared bits ─────────────────────────────────────────────────────────────

/// `c` with its alpha scaled by `k`.
fn fade(c: Color, k: f32) -> Color {
    Color {
        a: c.a * k.clamp(0.0, 1.0),
        ..c
    }
}

/// Dim factor for things that are *not* selected: 1.0 → 0.35 as the selection settles.
fn dim(sel_anim: f32) -> f32 {
    1.0 - 0.65 * sel_anim.clamp(0.0, 1.0)
}

// ── stacked bars ────────────────────────────────────────────────────────────

pub struct StackedBars {
    pub series: TimeSeries,
    pub metric: Metric,
    pub colors: ChartColors,
    /// Index of the selected bucket, if any.
    pub selected: Option<usize>,
    /// Entry animation, 0 → 1.
    pub anim: f32,
    /// Selection animation, 0 → 1.
    pub sel_anim: f32,
}

const LEFT: f32 = 58.0;
const BOTTOM: f32 = 24.0;
const TOP: f32 = 10.0;
const RIGHT: f32 = 10.0;

/// Plot-area size for a chart of `size`.
fn plot_size(size: Size) -> (f32, f32) {
    (
        (size.width - LEFT - RIGHT).max(1.0),
        (size.height - TOP - BOTTOM).max(1.0),
    )
}

/// Which bucket column the pointer is over (anywhere in the column counts, so
/// thin bars are easy to hit).
pub fn bar_at(size: Size, p: Point, buckets: usize) -> Option<usize> {
    let (plot_w, _) = plot_size(size);
    bucket_at(p.x - LEFT, plot_w, buckets)
}

/// Staggered, eased growth of bar `i` of `n` at entry-animation progress `anim`.
pub fn bar_progress(anim: f32, i: usize, n: usize) -> f32 {
    let stagger = 0.6 * i as f32 / n.max(1) as f32;
    crate::state::ease_out((anim * 1.6 - stagger).clamp(0.0, 1.0))
}

impl canvas::Program<Message> for StackedBars {
    type State = ();

    fn update(
        &self,
        _state: &mut (),
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            // Hovering moves the highlight and tooltip: repaint on every move.
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                Some(canvas::Action::request_redraw())
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let i = cursor
                    .position_in(bounds)
                    .and_then(|p| bar_at(bounds.size(), p, self.series.labels.len()))?;
                Some(
                    canvas::Action::publish(Message::BucketClicked(self.series.labels[i].clone()))
                        .and_capture(),
                )
            }
            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        _state: &(),
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let over = cursor
            .position_in(bounds)
            .and_then(|p| bar_at(bounds.size(), p, self.series.labels.len()))
            .is_some();
        if over {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let c = self.colors;
        let n = self.series.labels.len();
        let (plot_w, plot_h) = plot_size(bounds.size());

        if n == 0 {
            frame.fill_text(Text {
                content: "No data in this window".into(),
                position: Point::new(bounds.width / 2.0, bounds.height / 2.0),
                color: c.muted,
                size: Pixels(14.0),
                align_x: alignment::Horizontal::Center.into(),
                align_y: alignment::Vertical::Center,
                ..Text::default()
            });
            return vec![frame.into_geometry()];
        }

        let max = nice_max(self.series.max_total(self.metric));

        // Grid + y labels.
        for i in 0..=4 {
            let v = max * f64::from(i) / 4.0;
            let y = TOP + plot_h - (plot_h * i as f32 / 4.0);
            frame.stroke(
                &Path::line(Point::new(LEFT, y), Point::new(LEFT + plot_w, y)),
                Stroke::default().with_color(c.grid).with_width(1.0),
            );
            frame.fill_text(Text {
                content: axis_label(self.metric, v),
                position: Point::new(LEFT - 6.0, y),
                color: c.muted,
                size: Pixels(11.0),
                align_x: alignment::Horizontal::Right.into(),
                align_y: alignment::Vertical::Center,
                ..Text::default()
            });
        }

        // Stacked bars: they grow up from the axis, left to right.
        let slot = plot_w / n as f32;
        let bar_w = (slot * 0.78).max(1.0);
        let hover = cursor
            .position_in(bounds)
            .and_then(|p| bar_at(bounds.size(), p, n));
        let faded = self.selected.is_some();
        for i in 0..n {
            let x = LEFT + slot * i as f32 + (slot - bar_w) / 2.0;
            let grow = bar_progress(self.anim, i, n);
            let alpha = if faded && self.selected != Some(i) {
                dim(self.sel_anim)
            } else if hover.is_some() && hover != Some(i) && !faded {
                0.78
            } else {
                1.0
            };
            let mut y = TOP + plot_h;
            for tool in Tool::ALL {
                let v = self.series.value(self.metric, i, tool);
                if v <= 0.0 {
                    continue;
                }
                let h = (v / max) as f32 * plot_h * grow;
                y -= h;
                frame.fill_rectangle(
                    Point::new(x, y),
                    Size::new(bar_w, h.max(if grow > 0.0 { 1.0 } else { 0.0 })),
                    fade(c.series[tool.series_index() % 8], alpha),
                );
            }
            if self.selected == Some(i) || hover == Some(i) {
                let w = if self.selected == Some(i) { 2.0 } else { 1.0 };
                frame.stroke(
                    &Path::rectangle(Point::new(x - 1.0, TOP), Size::new(bar_w + 2.0, plot_h)),
                    Stroke::default().with_color(c.accent).with_width(w),
                );
            }
        }

        // X labels, thinned out so they never overlap.
        let every = ((72.0 / slot).ceil() as usize).max(1);
        for i in (0..n).step_by(every) {
            frame.fill_text(Text {
                content: short_label(&self.series.labels[i]),
                position: Point::new(LEFT + slot * (i as f32 + 0.5), TOP + plot_h + 6.0),
                color: c.muted,
                size: Pixels(11.0),
                align_x: alignment::Horizontal::Center.into(),
                ..Text::default()
            });
        }

        // Tooltip (once the bars have finished growing).
        if let (Some(i), Some(p), true) = (hover, cursor.position_in(bounds), self.anim >= 1.0) {
            let mut lines = vec![self.series.labels[i].clone()];
            for tool in self.series.active_tools(self.metric) {
                let v = self.series.value(self.metric, i, tool);
                if v > 0.0 {
                    lines.push(format!(
                        "{}  {}",
                        tool.display_name(),
                        axis_label(self.metric, v)
                    ));
                }
            }
            lines.push(format!(
                "total  {}",
                axis_label(self.metric, self.series.bucket_total(self.metric, i))
            ));
            lines.push(if self.selected == Some(i) {
                "click to clear".to_string()
            } else {
                "click to select".to_string()
            });
            let (w, h) = (170.0, 8.0 + 15.0 * lines.len() as f32);
            let x = (p.x + 12.0).min(bounds.width - w - 4.0).max(4.0);
            let y = (p.y - h - 8.0).max(4.0);
            frame.fill_rectangle(Point::new(x, y), Size::new(w, h), c.background);
            frame.stroke(
                &Path::rectangle(Point::new(x, y), Size::new(w, h)),
                Stroke::default().with_color(c.accent).with_width(1.0),
            );
            let last = lines.len() - 1;
            for (k, l) in lines.iter().enumerate() {
                frame.fill_text(Text {
                    content: l.clone(),
                    position: Point::new(x + 8.0, y + 6.0 + 15.0 * k as f32),
                    color: if k == 0 {
                        c.accent
                    } else if k == last {
                        c.muted
                    } else {
                        c.text
                    },
                    size: Pixels(12.0),
                    ..Text::default()
                });
            }
        }

        vec![frame.into_geometry()]
    }
}

/// The stacked-bar chart as a widget.
pub fn stacked_bars<'a>(
    series: TimeSeries,
    metric: Metric,
    colors: ChartColors,
    selected: Option<&str>,
    (anim, sel_anim): (f32, f32),
    height: f32,
) -> Element<'a, Message> {
    let selected = selected.and_then(|l| series.labels.iter().position(|x| x == l));
    Canvas::new(StackedBars {
        series,
        metric,
        colors,
        selected,
        anim,
        sel_anim,
    })
    .width(Length::Fill)
    .height(Length::Fixed(height))
    .into()
}

// ── donut ───────────────────────────────────────────────────────────────────

pub struct Donut {
    pub pie: Pie,
    pub colors: ChartColors,
    /// Index of the selected slice, if any.
    pub selected: Option<usize>,
    pub anim: f32,
    pub sel_anim: f32,
}

const ROW_H: f32 = 18.0;

/// Centre and outer radius of the ring in a chart of `size`.
pub fn donut_geometry(size: Size) -> (Point, f32) {
    let radius = (size.height.min(size.width * 0.5) / 2.0 - 8.0).max(10.0);
    (Point::new(radius + 10.0, size.height / 2.0), radius)
}

/// Which slice the point `p` is over (the ring, not the hole), `values` being
/// the slice values in drawing order.
pub fn slice_at(size: Size, p: Point, values: &[f64]) -> Option<usize> {
    let (center, radius) = donut_geometry(size);
    let (dx, dy) = (p.x - center.x, p.y - center.y);
    let r = (dx * dx + dy * dy).sqrt();
    if r > radius || r < radius * 0.58 {
        return None;
    }
    // Angle from 12 o'clock, clockwise, in [0, TAU).
    let angle = (dy.atan2(dx) + FRAC_PI_2).rem_euclid(TAU);
    slice_angles(values)
        .into_iter()
        .position(|(a, b)| angle >= a && angle < b)
}

/// Which legend row the point `p` is over.
pub fn legend_row_at(size: Size, p: Point, rows: usize) -> Option<usize> {
    let (center, radius) = donut_geometry(size);
    let lx = center.x + radius + 22.0;
    let top = (size.height - ROW_H * rows as f32).max(0.0) / 2.0;
    if p.x < lx - 4.0 || p.x > size.width || p.y < top {
        return None;
    }
    let i = ((p.y - top) / ROW_H) as usize;
    (i < rows).then_some(i)
}

impl Donut {
    fn values(&self) -> Vec<f64> {
        self.pie.slices.iter().map(|s| s.value).collect()
    }

    /// The slice under the cursor, via the ring or the legend.
    fn target(&self, size: Size, p: Point) -> Option<usize> {
        slice_at(size, p, &self.values()).or_else(|| legend_row_at(size, p, self.pie.slices.len()))
    }
}

impl canvas::Program<Message> for Donut {
    type State = ();

    fn update(
        &self,
        _state: &mut (),
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                Some(canvas::Action::request_redraw())
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let i = cursor
                    .position_in(bounds)
                    .and_then(|p| self.target(bounds.size(), p))?;
                // "other" has no key: it can't be selected.
                let key = self.pie.slices[i].key.clone()?;
                Some(canvas::Action::publish(Message::SliceClicked(key)).and_capture())
            }
            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        _state: &(),
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let selectable = cursor
            .position_in(bounds)
            .and_then(|p| self.target(bounds.size(), p))
            .is_some_and(|i| self.pie.slices[i].key.is_some());
        if selectable {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let c = self.colors;
        let total = self.pie.total();
        if total <= 0.0 {
            frame.fill_text(Text {
                content: "No data".into(),
                position: Point::new(bounds.width / 2.0, bounds.height / 2.0),
                color: c.muted,
                size: Pixels(14.0),
                align_x: alignment::Horizontal::Center.into(),
                align_y: alignment::Vertical::Center,
                ..Text::default()
            });
            return vec![frame.into_geometry()];
        }

        let (center, radius) = donut_geometry(bounds.size());
        let values = self.values();
        let angles = slice_angles(&values);
        let hover = cursor
            .position_in(bounds)
            .and_then(|p| self.target(bounds.size(), p));
        let faded = self.selected.is_some();
        // The ring sweeps round once on entry.
        let sweep = crate::state::ease_out(self.anim) * TAU;

        for (i, (a, b)) in angles.iter().copied().enumerate() {
            if a >= sweep {
                break;
            }
            let b = b.min(sweep);
            // 12 o'clock start, clockwise.
            let (start, end) = (a - FRAC_PI_2, b - FRAC_PI_2);
            let mid = (start + end) / 2.0;
            let pop = if self.selected == Some(i) {
                7.0 * self.sel_anim
            } else if hover == Some(i) {
                4.0
            } else {
                0.0
            };
            let o = Point::new(center.x + pop * mid.cos(), center.y + pop * mid.sin());
            let points = ring_segment(o, radius, radius * 0.58, start, end);
            let path = Path::new(|p| {
                p.move_to(points[0]);
                for pt in &points[1..] {
                    p.line_to(*pt);
                }
                p.close();
            });
            let alpha = if faded && self.selected != Some(i) {
                dim(self.sel_anim)
            } else {
                1.0
            };
            frame.fill(&path, fade(c.series[i % 8], alpha));
            if self.selected == Some(i) {
                frame.stroke(
                    &path,
                    Stroke::default().with_color(c.accent).with_width(1.5),
                );
            }
        }
        frame.fill_text(Text {
            content: axis_label(self.pie.metric, total),
            position: center,
            color: fade(c.text, crate::state::ease_out(self.anim)),
            size: Pixels(15.0),
            align_x: alignment::Horizontal::Center.into(),
            align_y: alignment::Vertical::Center,
            ..Text::default()
        });

        // Legend: colour chip, label, share. Hovered/selected rows get a backdrop.
        let lx = center.x + radius + 22.0;
        let top = (bounds.height - ROW_H * self.pie.slices.len() as f32).max(0.0) / 2.0;
        for (i, s) in self.pie.slices.iter().enumerate() {
            let y = top + ROW_H * i as f32;
            let alpha = if faded && self.selected != Some(i) {
                dim(self.sel_anim)
            } else {
                1.0
            };
            if hover == Some(i) || self.selected == Some(i) {
                frame.fill_rectangle(
                    Point::new(lx - 6.0, y),
                    Size::new(bounds.width - lx, ROW_H),
                    fade(c.accent, 0.18),
                );
            }
            frame.fill_rectangle(
                Point::new(lx, y + 4.0),
                Size::new(10.0, 10.0),
                fade(c.series[i % 8], alpha),
            );
            let pct = s.value / total * 100.0;
            frame.fill_text(Text {
                content: format!("{}  {pct:.0}%", s.label),
                position: Point::new(lx + 16.0, y + 2.0),
                color: fade(c.text, alpha),
                size: Pixels(12.0),
                max_width: (bounds.width - lx - 20.0).max(40.0),
                ..Text::default()
            });
        }
        vec![frame.into_geometry()]
    }
}

/// The share donut as a widget.
pub fn donut<'a>(
    pie: Pie,
    colors: ChartColors,
    selected: Option<&str>,
    (anim, sel_anim): (f32, f32),
    height: f32,
) -> Element<'a, Message> {
    let selected =
        selected.and_then(|k| pie.slices.iter().position(|s| s.key.as_deref() == Some(k)));
    Canvas::new(Donut {
        pie,
        colors,
        selected,
        anim,
        sel_anim,
    })
    .width(Length::Fill)
    .height(Length::Fixed(height))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_segments_stay_between_the_two_radii_and_close_up() {
        let c = Point::new(100.0, 100.0);
        let pts = ring_segment(c, 50.0, 30.0, 0.0, std::f32::consts::FRAC_PI_2);
        assert!(pts.len() >= 4);
        let r = |p: &Point| ((p.x - c.x).powi(2) + (p.y - c.y).powi(2)).sqrt();
        let (outer, inner) = pts.split_at(pts.len() / 2);
        assert!(
            outer.iter().all(|p| (r(p) - 50.0).abs() < 1e-3),
            "first half on the outer radius"
        );
        assert!(
            inner.iter().all(|p| (r(p) - 30.0).abs() < 1e-3),
            "second half on the inner radius"
        );
        // A tiny slice still yields a valid (>= 4 point) polygon.
        assert!(ring_segment(c, 50.0, 30.0, 0.0, 0.001).len() >= 4);
    }

    use iced::widget::canvas::Program as _;
    use tokenburn_core::Slice;

    #[test]
    fn bars_are_hit_by_column_and_outside_the_plot_is_a_miss() {
        let size = Size::new(458.0, 200.0); // plot is 390 wide → 10 buckets of 39 px
        assert_eq!(bar_at(size, Point::new(LEFT + 5.0, 50.0), 10), Some(0));
        assert_eq!(
            bar_at(size, Point::new(LEFT + 390.0 - 1.0, 50.0), 10),
            Some(9)
        );
        assert_eq!(
            bar_at(size, Point::new(LEFT - 5.0, 50.0), 10),
            None,
            "the y-axis gutter"
        );
        assert_eq!(
            bar_at(size, Point::new(LEFT + 5.0, 50.0), 0),
            None,
            "no data"
        );
    }

    #[test]
    fn bar_growth_is_staggered_and_finishes_everywhere() {
        assert_eq!(bar_progress(0.0, 0, 10), 0.0);
        assert!(
            bar_progress(0.3, 0, 10) > bar_progress(0.3, 9, 10),
            "left bars lead"
        );
        for i in 0..10 {
            assert!(
                (bar_progress(1.0, i, 10) - 1.0).abs() < 1e-6,
                "bar {i} must be full at the end"
            );
        }
    }

    fn pie_values() -> Vec<f64> {
        vec![50.0, 30.0, 20.0]
    }

    #[test]
    fn slices_are_hit_clockwise_from_twelve_oclock() {
        let size = Size::new(600.0, 200.0);
        let (c, r) = donut_geometry(size);
        let mid = r * 0.8; // inside the ring
                           // Just right of 12 o'clock is the first slice (0..50 %).
        assert_eq!(
            slice_at(size, Point::new(c.x + 3.0, c.y - mid), &pie_values()),
            Some(0)
        );
        // 3 o'clock is still the first (it covers 0°-180°).
        assert_eq!(
            slice_at(size, Point::new(c.x + mid, c.y), &pie_values()),
            Some(0)
        );
        // 7 o'clock lies in the second slice (180°-288°).
        assert_eq!(
            slice_at(
                size,
                Point::new(c.x - mid * 0.5, c.y + mid * 0.85),
                &pie_values()
            ),
            Some(1)
        );
        // 11 o'clock lies in the third (288°-360°).
        assert_eq!(
            slice_at(
                size,
                Point::new(c.x - mid * 0.6, c.y - mid * 0.8),
                &pie_values()
            ),
            Some(2)
        );
    }

    #[test]
    fn the_hole_and_the_outside_are_not_slices() {
        let size = Size::new(600.0, 200.0);
        let (c, r) = donut_geometry(size);
        assert_eq!(slice_at(size, c, &pie_values()), None, "the hole");
        assert_eq!(
            slice_at(size, Point::new(c.x + r + 5.0, c.y), &pie_values()),
            None
        );
    }

    #[test]
    fn legend_rows_are_hit_by_position() {
        let size = Size::new(600.0, 200.0);
        let (c, r) = donut_geometry(size);
        let lx = c.x + r + 22.0;
        let top = (size.height - ROW_H * 3.0) / 2.0;
        assert_eq!(
            legend_row_at(size, Point::new(lx + 20.0, top + 1.0), 3),
            Some(0)
        );
        assert_eq!(
            legend_row_at(size, Point::new(lx + 20.0, top + ROW_H * 2.5), 3),
            Some(2)
        );
        assert_eq!(
            legend_row_at(size, Point::new(lx + 20.0, top + ROW_H * 3.5), 3),
            None
        );
        assert_eq!(
            legend_row_at(size, Point::new(lx - 30.0, top + 1.0), 3),
            None,
            "left of the legend"
        );
    }

    #[test]
    fn selection_indices_are_resolved_from_labels_and_keys() {
        let colors = crate::features::theme::chart_colors(0);
        let pie = Pie {
            metric: Metric::Tokens,
            slices: vec![
                Slice {
                    label: "a".into(),
                    tool: None,
                    key: Some("pi:a".into()),
                    value: 2.0,
                },
                Slice {
                    label: "other".into(),
                    tool: None,
                    key: None,
                    value: 1.0,
                },
            ],
        };
        // Building the widgets with a selection (and with a stale one) must not panic.
        let _ = donut(pie.clone(), colors, Some("pi:a"), (1.0, 1.0), 200.0);
        let _ = donut(pie, colors, Some("pi:gone"), (0.4, 0.0), 200.0);
        let _ = stacked_bars(
            TimeSeries::default(),
            Metric::Tokens,
            colors,
            Some("x"),
            (0.0, 0.0),
            200.0,
        );
    }

    #[test]
    fn fading_scales_alpha_only() {
        let c = Color::from_rgba(0.2, 0.4, 0.6, 0.8);
        let f = fade(c, 0.5);
        assert_eq!((f.r, f.g, f.b), (c.r, c.g, c.b));
        assert!((f.a - 0.4).abs() < 1e-6);
        assert!(dim(0.0) > dim(1.0) && dim(1.0) > 0.0);
    }

    // ── the programs' event handling, driven with synthetic mouse events ──

    fn press() -> canvas::Event {
        canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
    }

    /// The message a program would publish for a left click at `p` (window-less:
    /// `bounds` sits at the origin so cursor and local coordinates coincide).
    fn clicked<P: canvas::Program<Message, State = ()>>(
        program: &P,
        bounds: Rectangle,
        p: Point,
    ) -> Option<Message> {
        program
            .update(&mut (), &press(), bounds, mouse::Cursor::Available(p))
            .and_then(|a| a.into_inner().0)
    }

    fn bars_program(selected: Option<usize>) -> StackedBars {
        let rows: Vec<tokenburn_core::Row> = ["2026-05-01", "2026-05-03", "2026-05-04"]
            .iter()
            .map(|d| tokenburn_core::Row {
                tool: Tool::Pi,
                project: "p".into(),
                id: "x".into(),
                ts: format!("{d}T12:00:00Z").parse().unwrap(),
                input: 10,
                output: 0,
                cache_read: 0,
                cache_write: 0,
                cost: 0.0,
            })
            .collect();
        let query = tokenburn_core::Query {
            bucket: tokenburn_core::Bucket::Day,
            window: tokenburn_core::Window::All,
            ..Default::default()
        };
        StackedBars {
            series: tokenburn_core::time_series(&rows, &query),
            metric: Metric::Tokens,
            colors: crate::features::theme::chart_colors(0),
            selected,
            anim: 1.0,
            sel_anim: 1.0,
        }
    }

    #[test]
    fn clicking_a_bar_column_publishes_that_bucket() {
        let prog = bars_program(None);
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(458.0, 200.0));
        let n = prog.series.labels.len();
        assert_eq!(n, 4, "May 1-4 with the gap day");
        let slot = (458.0 - LEFT - RIGHT) / n as f32;
        for i in 0..n {
            let p = Point::new(LEFT + slot * (i as f32 + 0.5), 100.0);
            match clicked(&prog, bounds, p) {
                Some(Message::BucketClicked(l)) => assert_eq!(l, prog.series.labels[i]),
                other => panic!("column {i}: {other:?}"),
            }
        }
    }

    #[test]
    fn clicks_outside_the_bars_publish_nothing() {
        let prog = bars_program(None);
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(458.0, 200.0));
        assert!(
            clicked(&prog, bounds, Point::new(10.0, 100.0)).is_none(),
            "y-axis gutter"
        );
        // The cursor outside the widget altogether.
        let off = prog.update(
            &mut (),
            &press(),
            bounds,
            mouse::Cursor::Available(Point::new(900.0, 100.0)),
        );
        assert!(off.is_none());
        assert!(prog
            .update(&mut (), &press(), bounds, mouse::Cursor::Unavailable)
            .is_none());
    }

    #[test]
    fn only_moves_and_left_presses_are_handled() {
        let prog = bars_program(None);
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(458.0, 200.0));
        let cur = mouse::Cursor::Available(Point::new(100.0, 100.0));
        let moved = canvas::Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(100.0, 100.0),
        });
        assert!(
            prog.update(&mut (), &moved, bounds, cur).is_some(),
            "hover repaints"
        );
        let right = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right));
        assert!(prog.update(&mut (), &right, bounds, cur).is_none());
        assert_eq!(
            prog.mouse_interaction(&(), bounds, cur),
            mouse::Interaction::Pointer
        );
        assert_eq!(
            prog.mouse_interaction(&(), bounds, mouse::Cursor::Available(Point::new(5.0, 5.0))),
            mouse::Interaction::default()
        );
    }

    fn donut_program() -> Donut {
        let slice = |label: &str, key: Option<&str>, value: f64| Slice {
            label: label.into(),
            tool: None,
            key: key.map(Into::into),
            value,
        };
        Donut {
            pie: Pie {
                metric: Metric::Tokens,
                slices: vec![
                    slice("pi:a", Some("pi:full-a"), 50.0),
                    slice("pi:b", Some("pi:full-b"), 30.0),
                    slice("other", None, 20.0),
                ],
            },
            colors: crate::features::theme::chart_colors(0),
            selected: None,
            anim: 1.0,
            sel_anim: 1.0,
        }
    }

    #[test]
    fn clicking_a_ring_slice_publishes_its_project_key() {
        let prog = donut_program();
        let size = Size::new(600.0, 200.0);
        let bounds = Rectangle::new(Point::ORIGIN, size);
        let (c, r) = donut_geometry(size);
        let mid = r * 0.8;
        match clicked(&prog, bounds, Point::new(c.x + 3.0, c.y - mid)) {
            Some(Message::SliceClicked(k)) => assert_eq!(k, "pi:full-a"),
            other => panic!("{other:?}"),
        }
        match clicked(&prog, bounds, Point::new(c.x - mid * 0.5, c.y + mid * 0.85)) {
            Some(Message::SliceClicked(k)) => assert_eq!(k, "pi:full-b"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn other_the_hole_and_empty_space_are_not_selectable() {
        let prog = donut_program();
        let size = Size::new(600.0, 200.0);
        let bounds = Rectangle::new(Point::ORIGIN, size);
        let (c, r) = donut_geometry(size);
        let mid = r * 0.8;
        // 11 o'clock is the folded "other" slice.
        assert!(clicked(&prog, bounds, Point::new(c.x - mid * 0.6, c.y - mid * 0.8)).is_none());
        assert!(clicked(&prog, bounds, c).is_none(), "the hole");
        assert!(
            clicked(&prog, bounds, Point::new(590.0, 5.0)).is_none(),
            "empty corner"
        );
    }

    #[test]
    fn clicking_a_legend_row_selects_the_same_project() {
        let prog = donut_program();
        let size = Size::new(600.0, 200.0);
        let bounds = Rectangle::new(Point::ORIGIN, size);
        let (c, r) = donut_geometry(size);
        let lx = c.x + r + 22.0;
        let top = (size.height - ROW_H * 3.0) / 2.0;
        match clicked(&prog, bounds, Point::new(lx + 30.0, top + ROW_H * 1.5)) {
            Some(Message::SliceClicked(k)) => assert_eq!(k, "pi:full-b"),
            other => panic!("{other:?}"),
        }
        assert!(
            clicked(&prog, bounds, Point::new(lx + 30.0, top + ROW_H * 2.5)).is_none(),
            "the 'other' row has no key"
        );
        assert_eq!(
            prog.mouse_interaction(
                &(),
                bounds,
                mouse::Cursor::Available(Point::new(lx + 30.0, top + 5.0))
            ),
            mouse::Interaction::Pointer
        );
        assert_eq!(
            prog.mouse_interaction(
                &(),
                bounds,
                mouse::Cursor::Available(Point::new(lx + 30.0, top + ROW_H * 2.5))
            ),
            mouse::Interaction::default(),
            "no pointer over 'other'"
        );
    }
}
