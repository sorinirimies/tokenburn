//! SVG charts, generated on the server from the same data the other front-ends
//! draw ([`TimeSeries`], [`Pie`] from `tokenburn-core`).
//!
//! Colours are CSS custom properties (`var(--s3)`, `var(--line)` …), so a theme
//! change restyles the charts without regenerating them. Every function here is
//! pure and returns markup built only from numbers and escaped labels.

use std::fmt::Write as _;

use tokenburn_core::{
    Metric, Pie, TimeSeries, Tool, axis_label, nice_max, short_label, slice_angles,
};

pub const BARS_W: f32 = 700.0;
pub const BARS_H: f32 = 240.0;
const LEFT: f32 = 56.0;
const RIGHT: f32 = 8.0;
const TOP: f32 = 10.0;
const BOTTOM: f32 = 26.0;

/// Escape text for HTML/SVG content and attribute values.
pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// `var(--sN)` for a tool.
pub fn tool_var(tool: Tool) -> String {
    format!("var(--s{})", tool.series_index() % 8)
}

/// Stacked bars per bucket, one coloured segment per tool, with grid, axis
/// labels and a `<title>` tooltip on every segment (native hover).
pub fn bars_svg(series: &TimeSeries, metric: Metric) -> String {
    let mut s = String::new();
    let _ = write!(
        s,
        r#"<svg class="chart" viewBox="0 0 {BARS_W} {BARS_H}" width="100%" role="img" aria-label="{} over time" xmlns="http://www.w3.org/2000/svg">"#,
        metric.label()
    );
    let n = series.labels.len();
    if n == 0 {
        let _ = write!(
            s,
            r#"<text x="{}" y="{}" fill="var(--dim)" font-size="14" text-anchor="middle">No data in this window</text></svg>"#,
            BARS_W / 2.0,
            BARS_H / 2.0
        );
        return s;
    }
    let plot_w = BARS_W - LEFT - RIGHT;
    let plot_h = BARS_H - TOP - BOTTOM;
    let max = nice_max(series.max_total(metric));

    // Grid + y labels.
    for i in 0..=4 {
        let y = TOP + plot_h - plot_h * i as f32 / 4.0;
        let v = max * f64::from(i) / 4.0;
        let _ = write!(
            s,
            r#"<line x1="{LEFT}" x2="{}" y1="{y:.1}" y2="{y:.1}" stroke="var(--line)" stroke-width="1"/><text x="{}" y="{:.1}" fill="var(--dim)" font-size="12" text-anchor="end">{}</text>"#,
            LEFT + plot_w,
            LEFT - 6.0,
            y + 4.0,
            esc(&axis_label(metric, v))
        );
    }

    // Bars.
    let slot = plot_w / n as f32;
    let bar_w = (slot * 0.78).max(1.0);
    for i in 0..n {
        let x = LEFT + slot * i as f32 + (slot - bar_w) / 2.0;
        let mut y = TOP + plot_h;
        let total = series.bucket_total(metric, i);
        for tool in Tool::ALL {
            let v = series.value(metric, i, tool);
            if v <= 0.0 {
                continue;
            }
            let h = (v / max) as f32 * plot_h;
            y -= h;
            let _ = write!(
                s,
                r#"<rect x="{x:.1}" y="{y:.1}" width="{bar_w:.1}" height="{:.1}" fill="{}"><title>{} · {} — {} (bucket total {})</title></rect>"#,
                h.max(1.0),
                tool_var(tool),
                esc(&series.labels[i]),
                esc(tool.display_name()),
                esc(&axis_label(metric, v)),
                esc(&axis_label(metric, total)),
            );
        }
    }

    // X labels, thinned so they never overlap.
    let every = ((72.0 / slot).ceil() as usize).max(1);
    for i in (0..n).step_by(every) {
        let _ = write!(
            s,
            r#"<text x="{:.1}" y="{:.1}" fill="var(--dim)" font-size="12" text-anchor="middle">{}</text>"#,
            LEFT + slot * (i as f32 + 0.5),
            TOP + plot_h + 16.0,
            esc(&short_label(&series.labels[i]))
        );
    }
    s.push_str("</svg>");
    s
}

pub const DONUT: f32 = 200.0;

/// SVG path for one ring segment from `start` to `end` radians (clockwise from
/// 12 o'clock) between radii `r_in` and `r_out`.
pub fn ring_path(cx: f32, cy: f32, r_out: f32, r_in: f32, start: f32, end: f32) -> String {
    // A full circle is a degenerate arc: stop a hair short of closing it.
    let end = end.min(start + std::f32::consts::TAU - 0.0005);
    let pt = |r: f32, a: f32| {
        let a = a - std::f32::consts::FRAC_PI_2;
        (cx + r * a.cos(), cy + r * a.sin())
    };
    let large = u8::from(end - start > std::f32::consts::PI);
    let (x0, y0) = pt(r_out, start);
    let (x1, y1) = pt(r_out, end);
    let (x2, y2) = pt(r_in, end);
    let (x3, y3) = pt(r_in, start);
    format!(
        "M{x0:.2} {y0:.2} A{r_out:.2} {r_out:.2} 0 {large} 1 {x1:.2} {y1:.2} L{x2:.2} {y2:.2} A{r_in:.2} {r_in:.2} 0 {large} 0 {x3:.2} {y3:.2} Z"
    )
}

/// The share donut with its total in the middle.
pub fn donut_svg(pie: &Pie) -> String {
    let total = pie.total();
    let mut s = format!(
        r#"<svg class="chart donut" viewBox="0 0 {DONUT} {DONUT}" width="100%" role="img" aria-label="share by project" xmlns="http://www.w3.org/2000/svg">"#
    );
    if total <= 0.0 {
        let _ = write!(
            s,
            r#"<text x="100" y="104" fill="var(--dim)" font-size="13" text-anchor="middle">No data</text></svg>"#
        );
        return s;
    }
    let values: Vec<f64> = pie.slices.iter().map(|x| x.value).collect();
    for (i, (a, b)) in slice_angles(&values).into_iter().enumerate() {
        let slice = &pie.slices[i];
        let _ = write!(
            s,
            r#"<path d="{}" fill="var(--s{})"><title>{} — {} ({:.0}%)</title></path>"#,
            ring_path(100.0, 100.0, 92.0, 56.0, a, b),
            i % 8,
            esc(&slice.label),
            esc(&axis_label(pie.metric, slice.value)),
            slice.value / total * 100.0
        );
    }
    let _ = write!(
        s,
        r#"<text x="100" y="106" fill="var(--fg)" font-size="17" text-anchor="middle">{}</text></svg>"#,
        esc(&axis_label(pie.metric, total))
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokenburn_core::{Bucket, Query, Row, Slice, Window};

    fn row(tool: Tool, ts: &str, tokens: u64) -> Row {
        Row {
            tool,
            project: "p".into(),
            id: "i".into(),
            ts: ts.parse().unwrap(),
            input: tokens,
            output: 0,
            cache_read: 0,
            cache_write: 0,
            cost: tokens as f64 / 1000.0,
        }
    }

    fn series() -> TimeSeries {
        let rows = [
            row(Tool::Pi, "2026-05-10T12:00:00Z", 1000),
            row(Tool::Claude, "2026-05-10T13:00:00Z", 500),
            row(Tool::Pi, "2026-05-12T12:00:00Z", 2000),
        ];
        tokenburn_core::time_series(
            &rows,
            &Query {
                bucket: Bucket::Day,
                window: Window::Month,
                ..Query::default()
            },
        )
    }

    #[test]
    fn escaping_covers_the_html_metacharacters() {
        assert_eq!(
            esc(r#"<a href="x">&'"#),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;"
        );
    }

    #[test]
    fn bars_have_a_rect_per_tool_segment_with_tooltips_and_theme_colours() {
        let svg = bars_svg(&series(), Metric::Tokens);
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        assert_eq!(
            svg.matches("<rect").count(),
            3,
            "pi+claude on day 1, pi on day 3"
        );
        assert!(svg.contains("var(--s0)"), "pi uses its series colour");
        assert!(svg.contains(&tool_var(Tool::Claude)));
        assert!(svg.contains("<title>"), "native hover tooltips");
        assert!(svg.contains("Claude Code"));
        assert!(svg.contains("stroke=\"var(--line)\"") && svg.contains("fill=\"var(--dim)\""));
    }

    #[test]
    fn bars_are_scaled_to_a_nice_axis_maximum() {
        let svg = bars_svg(&series(), Metric::Tokens);
        // The tallest bucket is 2000 tokens → the axis tops out at a "nice" 2k.
        assert!(svg.contains(">2k<"), "{svg}");
        assert!(svg.contains(">0<"));
    }

    #[test]
    fn the_cost_metric_draws_only_tools_that_have_a_cost() {
        let svg = bars_svg(&series(), Metric::Cost);
        assert!(svg.contains('$'), "dollar axis");
        assert_eq!(
            svg.matches("<rect").count(),
            3,
            "every tool in this fixture has a cost"
        );
    }

    #[test]
    fn an_empty_series_is_a_message_not_a_broken_chart() {
        let svg = bars_svg(&TimeSeries::default(), Metric::Tokens);
        assert!(svg.contains("No data in this window"));
        assert!(!svg.contains("<rect"));
    }

    #[test]
    fn rects_stay_inside_the_viewbox() {
        let svg = bars_svg(&series(), Metric::Tokens);
        for cap in svg.split("<rect x=\"").skip(1) {
            let x: f32 = cap.split('"').next().unwrap().parse().unwrap();
            assert!((0.0..BARS_W).contains(&x), "x={x}");
        }
    }

    #[test]
    fn labels_cannot_inject_markup() {
        let pie = Pie {
            metric: Metric::Tokens,
            slices: vec![Slice {
                label: "<script>alert(1)</script>".into(),
                tool: None,
                value: 5.0,
            }],
        };
        let svg = donut_svg(&pie);
        assert!(!svg.contains("<script>"));
        assert!(svg.contains("&lt;script&gt;"));
    }

    #[test]
    fn donut_has_one_path_per_slice_and_the_total_in_the_middle() {
        let pie = Pie {
            metric: Metric::Cost,
            slices: vec![
                Slice {
                    label: "pi:a".into(),
                    tool: Some(Tool::Pi),
                    value: 30.0,
                },
                Slice {
                    label: "pi:b".into(),
                    tool: Some(Tool::Pi),
                    value: 10.0,
                },
            ],
        };
        let svg = donut_svg(&pie);
        assert_eq!(svg.matches("<path").count(), 2);
        assert!(svg.contains(">$40.00<"), "{svg}");
        assert!(svg.contains("75%") && svg.contains("25%"));
        assert!(svg.contains("var(--s0)") && svg.contains("var(--s1)"));
    }

    #[test]
    fn a_single_slice_still_draws_a_closed_ring() {
        let pie = Pie {
            metric: Metric::Tokens,
            slices: vec![Slice {
                label: "only".into(),
                tool: None,
                value: 1.0,
            }],
        };
        let svg = donut_svg(&pie);
        assert_eq!(svg.matches("<path").count(), 1);
        assert!(!svg.contains("NaN") && !svg.contains("inf"));
    }

    #[test]
    fn an_empty_pie_says_so() {
        let svg = donut_svg(&Pie {
            metric: Metric::Tokens,
            slices: vec![],
        });
        assert!(svg.contains("No data") && !svg.contains("<path"));
    }

    #[test]
    fn ring_paths_use_the_large_arc_flag_past_half_a_circle() {
        let small = ring_path(100.0, 100.0, 90.0, 50.0, 0.0, 1.0);
        let big = ring_path(100.0, 100.0, 90.0, 50.0, 0.0, 4.0);
        assert!(small.contains(" 0 0 1 "), "{small}");
        assert!(big.contains(" 0 1 1 "), "{big}");
        assert!(small.starts_with('M') && small.ends_with('Z'));
    }
}
