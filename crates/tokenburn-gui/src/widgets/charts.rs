//! Charts drawn on an iced canvas: stacked bars over time and a share donut.
//!
//! Both are plain [`canvas::Program`]s over data that `tokenburn-core` builds
//! ([`TimeSeries`], [`Pie`]), coloured from the active theme.

use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke, Text};
use iced::{alignment, Color, Element, Length, Pixels, Point, Rectangle, Renderer, Size, Theme};
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

// ── stacked bars ────────────────────────────────────────────────────────────

pub struct StackedBars {
    pub series: TimeSeries,
    pub metric: Metric,
    pub colors: ChartColors,
}

const LEFT: f32 = 58.0;
const BOTTOM: f32 = 24.0;
const TOP: f32 = 10.0;
const RIGHT: f32 = 10.0;

impl canvas::Program<Message> for StackedBars {
    type State = ();

    fn update(
        &self,
        _state: &mut (),
        event: &canvas::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        // Hovering moves the tooltip: ask for a repaint on every cursor move.
        matches!(
            event,
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. })
        )
        .then(canvas::Action::request_redraw)
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
        let plot_w = (bounds.width - LEFT - RIGHT).max(1.0);
        let plot_h = (bounds.height - TOP - BOTTOM).max(1.0);

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

        // Stacked bars.
        let slot = plot_w / n as f32;
        let bar_w = (slot * 0.78).max(1.0);
        let hover = cursor
            .position_in(bounds)
            .and_then(|p| bucket_at(p.x - LEFT, plot_w, n));
        for i in 0..n {
            let x = LEFT + slot * i as f32 + (slot - bar_w) / 2.0;
            let mut y = TOP + plot_h;
            for tool in Tool::ALL {
                let v = self.series.value(self.metric, i, tool);
                if v <= 0.0 {
                    continue;
                }
                let h = (v / max) as f32 * plot_h;
                y -= h;
                frame.fill_rectangle(
                    Point::new(x, y),
                    Size::new(bar_w, h.max(1.0)),
                    c.series[tool.series_index() % 8],
                );
            }
            if hover == Some(i) {
                frame.stroke(
                    &Path::rectangle(Point::new(x - 1.0, TOP), Size::new(bar_w + 2.0, plot_h)),
                    Stroke::default().with_color(c.accent).with_width(1.0),
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

        // Tooltip.
        if let (Some(i), Some(p)) = (hover, cursor.position_in(bounds)) {
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
            let (w, h) = (170.0, 8.0 + 15.0 * lines.len() as f32);
            let x = (p.x + 12.0).min(bounds.width - w - 4.0).max(4.0);
            let y = (p.y - h - 8.0).max(4.0);
            frame.fill_rectangle(Point::new(x, y), Size::new(w, h), c.background);
            frame.stroke(
                &Path::rectangle(Point::new(x, y), Size::new(w, h)),
                Stroke::default().with_color(c.accent).with_width(1.0),
            );
            for (k, l) in lines.iter().enumerate() {
                frame.fill_text(Text {
                    content: l.clone(),
                    position: Point::new(x + 8.0, y + 6.0 + 15.0 * k as f32),
                    color: if k == 0 { c.accent } else { c.text },
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
    height: f32,
) -> Element<'a, Message> {
    Canvas::new(StackedBars {
        series,
        metric,
        colors,
    })
    .width(Length::Fill)
    .height(Length::Fixed(height))
    .into()
}

// ── donut ───────────────────────────────────────────────────────────────────

pub struct Donut {
    pub pie: Pie,
    pub colors: ChartColors,
}

impl canvas::Program<Message> for Donut {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
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

        let radius = (bounds.height.min(bounds.width * 0.5) / 2.0 - 8.0).max(10.0);
        let center = Point::new(radius + 10.0, bounds.height / 2.0);
        let values: Vec<f64> = self.pie.slices.iter().map(|s| s.value).collect();
        let tau_quarter = std::f32::consts::FRAC_PI_2;

        for (i, (a, b)) in slice_angles(&values).into_iter().enumerate() {
            // 12 o'clock start, clockwise.
            let (start, end) = (a - tau_quarter, b - tau_quarter);
            let points = ring_segment(center, radius, radius * 0.58, start, end);
            let path = Path::new(|p| {
                p.move_to(points[0]);
                for pt in &points[1..] {
                    p.line_to(*pt);
                }
                p.close();
            });
            frame.fill(&path, c.series[i % 8]);
        }
        frame.fill_text(Text {
            content: axis_label(self.pie.metric, total),
            position: center,
            color: c.text,
            size: Pixels(15.0),
            align_x: alignment::Horizontal::Center.into(),
            align_y: alignment::Vertical::Center,
            ..Text::default()
        });

        // Legend: colour chip, label, share.
        let lx = center.x + radius + 22.0;
        let row_h = 18.0;
        let top = (bounds.height - row_h * self.pie.slices.len() as f32).max(0.0) / 2.0;
        for (i, s) in self.pie.slices.iter().enumerate() {
            let y = top + row_h * i as f32;
            frame.fill_rectangle(
                Point::new(lx, y + 3.0),
                Size::new(10.0, 10.0),
                c.series[i % 8],
            );
            let pct = s.value / total * 100.0;
            frame.fill_text(Text {
                content: format!("{}  {pct:.0}%", s.label),
                position: Point::new(lx + 16.0, y),
                color: c.text,
                size: Pixels(12.0),
                max_width: (bounds.width - lx - 20.0).max(40.0),
                ..Text::default()
            });
        }
        vec![frame.into_geometry()]
    }
}

/// The share donut as a widget.
pub fn donut<'a>(pie: Pie, colors: ChartColors, height: f32) -> Element<'a, Message> {
    Canvas::new(Donut { pie, colors })
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
}
