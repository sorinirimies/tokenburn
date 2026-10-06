//! Chart geometry for the web dashboard: stacked bars and a donut.
//!
//! Everything here is **pure** — it turns [`TimeSeries`] / [`Pie`] into numbers,
//! paths and labels (viewBox units). `components.rs` renders them as inline SVG,
//! attaching a click handler to each bar and slice, and `style.rs` animates and
//! highlights them with CSS.
//!
//! Colours are CSS custom properties (`var(--s3)`, `var(--line)` …), so a theme
//! change restyles the charts without regenerating them.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use tokenburn_core::utils::fmt::percent;
use tokenburn_core::{
    Metric, Pie, Selection, TimeSeries, Tool, axis_label, nice_max, short_label, slice_angles,
};

pub const BARS_W: f32 = 700.0;
pub const BARS_H: f32 = 240.0;
pub const LEFT: f32 = 56.0;
pub const RIGHT: f32 = 8.0;
pub const TOP: f32 = 10.0;
pub const BOTTOM: f32 = 26.0;

/// Escape text for HTML/SVG content and attribute values.
pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Percent-encode `s` for use as a URL query value.
pub fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// `var(--sN)` for a tool.
pub fn tool_var(tool: Tool) -> String {
    format!("var(--s{})", tool.series_index() % 8)
}

fn f(v: f32) -> String {
    format!("{v:.1}")
}

// ── bars ────────────────────────────────────────────────────────────────────

/// One tool's segment of a stacked bar.
#[derive(Debug, Clone, PartialEq)]
pub struct Seg {
    pub x: String,
    pub y: String,
    pub w: String,
    pub h: String,
    pub var: String,
}

/// One bucket: the stacked segments plus the full-height hit area.
#[derive(Debug, Clone, PartialEq)]
pub struct Col {
    pub index: usize,
    pub label: String,
    /// Native tooltip (`<title>`): the bucket, every tool's value and the total.
    pub title: String,
    pub selected: bool,
    /// Left edge and width of the whole column (the click target).
    pub slot_x: String,
    pub slot_w: String,
    pub segs: Vec<Seg>,
    /// Axis position of the plot's top/height (for the hit rect).
    pub plot_y: String,
    pub plot_h: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridLine {
    pub y: String,
    pub text_y: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct XLabel {
    pub x: String,
    pub y: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bars {
    pub empty: bool,
    pub grid: Vec<GridLine>,
    pub cols: Vec<Col>,
    pub x_labels: Vec<XLabel>,
    /// Whether any bar is selected (the others dim).
    pub has_selection: bool,
    pub left: String,
    pub right: String,
}

/// Lay out `series` for `metric`; `selected` is the selected bucket label.
pub fn bars(series: &TimeSeries, metric: Metric, selected: Option<&str>) -> Bars {
    let n = series.labels.len();
    let mut out = Bars {
        empty: n == 0,
        grid: Vec::new(),
        cols: Vec::new(),
        x_labels: Vec::new(),
        has_selection: selected.is_some_and(|s| series.labels.iter().any(|l| l == s)),
        left: f(LEFT),
        right: f(BARS_W - RIGHT),
    };
    if n == 0 {
        return out;
    }
    let plot_w = BARS_W - LEFT - RIGHT;
    let plot_h = BARS_H - TOP - BOTTOM;
    let max = nice_max(series.max_total(metric));

    for i in 0..=4 {
        let y = TOP + plot_h - plot_h * i as f32 / 4.0;
        out.grid.push(GridLine {
            y: f(y),
            text_y: f(y + 4.0),
            label: axis_label(metric, max * f64::from(i) / 4.0),
        });
    }

    let slot = plot_w / n as f32;
    let bar_w = (slot * 0.78).max(1.0);
    for i in 0..n {
        let x = LEFT + slot * i as f32 + (slot - bar_w) / 2.0;
        let mut y = TOP + plot_h;
        let mut lines = vec![series.labels[i].clone()];
        let mut segs = Vec::new();
        for tool in Tool::ALL {
            let v = series.value(metric, i, tool);
            if v <= 0.0 {
                continue;
            }
            let h = (v / max) as f32 * plot_h;
            y -= h;
            lines.push(format!(
                "{}: {}",
                tool.display_name(),
                axis_label(metric, v)
            ));
            segs.push(Seg {
                x: f(x),
                y: f(y),
                w: f(bar_w),
                h: f(h.max(1.0)),
                var: tool_var(tool),
            });
        }
        lines.push(format!(
            "total: {}",
            axis_label(metric, series.bucket_total(metric, i))
        ));
        let is_sel = selected == Some(series.labels[i].as_str());
        lines.push(if is_sel {
            "click to clear".into()
        } else {
            "click to select".into()
        });
        out.cols.push(Col {
            index: i,
            label: series.labels[i].clone(),
            title: lines.join("\n"),
            selected: is_sel,
            slot_x: f(LEFT + slot * i as f32),
            slot_w: f(slot),
            segs,
            plot_y: f(TOP),
            plot_h: f(plot_h),
        });
    }

    // X labels, thinned so they never overlap.
    let every = ((72.0 / slot).ceil() as usize).max(1);
    for i in (0..n).step_by(every) {
        out.x_labels.push(XLabel {
            x: f(LEFT + slot * (i as f32 + 0.5)),
            y: f(TOP + plot_h + 16.0),
            text: short_label(&series.labels[i]),
        });
    }
    out
}

// ── donut ───────────────────────────────────────────────────────────────────

pub const DONUT: f32 = 200.0;
const R_OUT: f32 = 92.0;
const R_IN: f32 = 56.0;
/// How far a hovered/selected slice moves out, in viewBox units.
const POP: f32 = 6.0;

/// SVG path for one ring segment from `start` to `end` radians (clockwise from
/// 12 o'clock) between radii `r_in` and `r_out`.
pub fn ring_path(cx: f32, cy: f32, r_out: f32, r_in: f32, start: f32, end: f32) -> String {
    // A full circle is a degenerate arc: stop a hair short of closing it.
    let end = end.min(start + TAU - 0.0005);
    let pt = |r: f32, a: f32| {
        let a = a - FRAC_PI_2;
        (cx + r * a.cos(), cy + r * a.sin())
    };
    let large = u8::from(end - start > PI);
    let (x0, y0) = pt(r_out, start);
    let (x1, y1) = pt(r_out, end);
    let (x2, y2) = pt(r_in, end);
    let (x3, y3) = pt(r_in, start);
    format!(
        "M{x0:.2} {y0:.2} A{r_out:.2} {r_out:.2} 0 {large} 1 {x1:.2} {y1:.2} L{x2:.2} {y2:.2} A{r_in:.2} {r_in:.2} 0 {large} 0 {x3:.2} {y3:.2} Z"
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct SliceGeom {
    pub index: usize,
    pub label: String,
    /// Full `tool:project` key; `None` for `other` (not selectable).
    pub key: Option<String>,
    pub path: String,
    pub var: String,
    pub title: String,
    pub pct: String,
    pub selected: bool,
    /// Hover/selected offset (CSS custom properties `--dx`/`--dy`, along the slice's middle).
    pub dx: String,
    pub dy: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Donut {
    pub empty: bool,
    pub slices: Vec<SliceGeom>,
    pub total_label: String,
    pub has_selection: bool,
}

/// Lay out `pie`; `selected` is the selected project key.
pub fn donut(pie: &Pie, selected: Option<&str>) -> Donut {
    let total = pie.total();
    let mut out = Donut {
        empty: total <= 0.0,
        slices: Vec::new(),
        total_label: axis_label(pie.metric, total),
        has_selection: selected
            .is_some_and(|k| pie.slices.iter().any(|s| s.key.as_deref() == Some(k))),
    };
    if out.empty {
        return out;
    }
    let values: Vec<f64> = pie.slices.iter().map(|s| s.value).collect();
    for (i, (a, b)) in slice_angles(&values).into_iter().enumerate() {
        let s = &pie.slices[i];
        let mid = (a + b) / 2.0 - FRAC_PI_2;
        let is_sel = s.key.is_some() && s.key.as_deref() == selected;
        let pct = percent(s.value / total);
        out.slices.push(SliceGeom {
            index: i,
            label: s.label.clone(),
            key: s.key.clone(),
            path: ring_path(100.0, 100.0, R_OUT, R_IN, a, b),
            var: format!("var(--s{})", i % 8),
            title: format!(
                "{}\n{} ({pct})\n{}",
                s.label,
                axis_label(pie.metric, s.value),
                match (&s.key, is_sel) {
                    (None, _) => "(several small projects)",
                    (_, true) => "click to clear",
                    _ => "click to select",
                }
            ),
            pct,
            selected: is_sel,
            dx: f(POP * mid.cos()),
            dy: f(POP * mid.sin()),
        });
    }
    out
}

// ── links (static mode) ─────────────────────────────────────────────────────

/// `base` (a `/?…` URL) with the selection appended.
pub fn href(base: &str, sel: &Selection) -> String {
    let mut url = base.to_string();
    if let Some(b) = &sel.bucket {
        url.push_str("&sel_bucket=");
        url.push_str(&encode(b));
    }
    if let Some(p) = &sel.project {
        url.push_str("&sel_project=");
        url.push_str(&encode(p));
    }
    url
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

    fn slice(label: &str, key: Option<&str>, value: f64) -> Slice {
        Slice {
            label: label.into(),
            tool: None,
            key: key.map(Into::into),
            value,
        }
    }

    #[test]
    fn escaping_and_url_encoding() {
        assert_eq!(
            esc(r#"<a href="x">&'"#),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;"
        );
        assert_eq!(encode("pi:atlas v2/é"), "pi%3Aatlas%20v2%2F%C3%A9");
        assert_eq!(encode("2026-09-22_a.b~"), "2026-09-22_a.b~");
    }

    #[test]
    fn there_is_a_column_per_bucket_and_a_segment_per_tool() {
        let b = bars(&series(), Metric::Tokens, None);
        assert!(!b.empty);
        assert_eq!(b.cols.len(), 3, "May 10, 11 (gap) and 12");
        assert_eq!(b.cols[0].segs.len(), 2, "pi + claude");
        assert_eq!(b.cols[1].segs.len(), 0, "the gap day");
        assert_eq!(b.cols[2].segs.len(), 1);
        assert!(b.cols[0].segs[0].var.starts_with("var(--s"));
        assert!(!b.has_selection && b.cols.iter().all(|c| !c.selected));
    }

    #[test]
    fn tooltips_list_the_tools_and_the_total_and_the_click_hint() {
        let b = bars(&series(), Metric::Tokens, None);
        let t = &b.cols[0].title;
        assert!(
            t.contains("Claude Code") && t.contains("total: 1.5k"),
            "{t}"
        );
        assert!(t.ends_with("click to select"));
    }

    #[test]
    fn the_selected_bucket_is_flagged_and_its_hint_flips() {
        let s = series();
        let label = s.labels[2].clone();
        let b = bars(&s, Metric::Tokens, Some(&label));
        assert!(b.has_selection);
        assert!(b.cols[2].selected && !b.cols[0].selected);
        assert!(b.cols[2].title.ends_with("click to clear"));
        // A stale label selects nothing.
        assert!(!bars(&s, Metric::Tokens, Some("1999-01-01")).has_selection);
    }

    #[test]
    fn the_axis_tops_out_at_a_nice_maximum() {
        let b = bars(&series(), Metric::Tokens, None);
        assert_eq!(b.grid.len(), 5);
        assert_eq!(b.grid[0].label, "0");
        assert_eq!(b.grid[4].label, "2k");
    }

    #[test]
    fn the_cost_metric_uses_dollars() {
        let b = bars(&series(), Metric::Cost, None);
        assert!(b.grid[4].label.starts_with('$'));
    }

    #[test]
    fn an_empty_series_is_flagged_not_drawn() {
        let b = bars(&TimeSeries::default(), Metric::Tokens, None);
        assert!(b.empty && b.cols.is_empty() && b.grid.is_empty());
    }

    #[test]
    fn everything_stays_inside_the_viewbox() {
        for c in bars(&series(), Metric::Tokens, None).cols {
            for s in c.segs {
                let (x, w): (f32, f32) = (s.x.parse().unwrap(), s.w.parse().unwrap());
                assert!(x >= LEFT && x + w <= BARS_W - RIGHT + 0.1, "{x} {w}");
                let (y, h): (f32, f32) = (s.y.parse().unwrap(), s.h.parse().unwrap());
                assert!(y >= TOP - 0.1 && y + h <= BARS_H - BOTTOM + 1.1, "{y} {h}");
            }
        }
    }

    #[test]
    fn x_labels_are_thinned_and_shortened() {
        let b = bars(&series(), Metric::Tokens, None);
        assert!(!b.x_labels.is_empty() && b.x_labels.len() <= b.cols.len());
        assert!(b.x_labels[0].text.len() <= 5, "{}", b.x_labels[0].text);
    }

    #[test]
    fn donut_slices_carry_keys_percentages_and_theme_colours() {
        let pie = Pie {
            metric: Metric::Cost,
            slices: vec![
                slice("pi:a", Some("pi:full-a"), 30.0),
                slice("pi:b", Some("pi:full-b"), 10.0),
            ],
        };
        let d = donut(&pie, None);
        assert_eq!(d.slices.len(), 2);
        assert_eq!(d.total_label, "$40.0");
        assert_eq!(d.slices[0].pct, "75%");
        assert_eq!(d.slices[0].var, "var(--s0)");
        assert_eq!(d.slices[1].var, "var(--s1)");
        assert_eq!(d.slices[0].key.as_deref(), Some("pi:full-a"));
        assert!(d.slices[0].title.contains("click to select"));
        assert!(!d.has_selection);
    }

    #[test]
    fn the_selected_slice_is_flagged_and_other_is_not_selectable() {
        let pie = Pie {
            metric: Metric::Tokens,
            slices: vec![slice("pi:a", Some("pi:a"), 3.0), slice("other", None, 1.0)],
        };
        let d = donut(&pie, Some("pi:a"));
        assert!(d.has_selection && d.slices[0].selected && !d.slices[1].selected);
        assert!(d.slices[0].title.contains("click to clear"));
        assert!(d.slices[1].title.contains("several small projects"));
        assert!(!donut(&pie, Some("pi:gone")).has_selection);
    }

    #[test]
    fn slices_pop_outwards_along_their_middle() {
        let pie = Pie {
            metric: Metric::Tokens,
            slices: vec![slice("a", Some("a"), 1.0), slice("b", Some("b"), 1.0)],
        };
        let d = donut(&pie, None);
        // First half is the right-hand side (12 → 6 o'clock): offset points right; the other left.
        assert!(d.slices[0].dx.parse::<f32>().unwrap() > 0.0);
        assert!(d.slices[1].dx.parse::<f32>().unwrap() < 0.0);
        for s in &d.slices {
            let (dx, dy): (f32, f32) = (s.dx.parse().unwrap(), s.dy.parse().unwrap());
            assert!(((dx * dx + dy * dy).sqrt() - POP).abs() < 0.2);
        }
    }

    #[test]
    fn a_single_slice_still_draws_a_closed_ring() {
        let pie = Pie {
            metric: Metric::Tokens,
            slices: vec![slice("only", Some("only"), 1.0)],
        };
        let d = donut(&pie, None);
        assert_eq!(d.slices.len(), 1);
        assert!(!d.slices[0].path.contains("NaN") && !d.slices[0].path.contains("inf"));
    }

    #[test]
    fn an_empty_pie_is_flagged() {
        assert!(
            donut(
                &Pie {
                    metric: Metric::Tokens,
                    slices: vec![]
                },
                None
            )
            .empty
        );
    }

    #[test]
    fn ring_paths_use_the_large_arc_flag_past_half_a_circle() {
        let small = ring_path(100.0, 100.0, 90.0, 50.0, 0.0, 1.0);
        let big = ring_path(100.0, 100.0, 90.0, 50.0, 0.0, 4.0);
        assert!(small.contains(" 0 0 1 "), "{small}");
        assert!(big.contains(" 0 1 1 "), "{big}");
        assert!(small.starts_with('M') && small.ends_with('Z'));
    }

    #[test]
    fn links_append_the_selection_encoded() {
        let base = "/?tool=pi&bucket=daily";
        assert_eq!(href(base, &Selection::default()), base);
        let sel = Selection {
            bucket: Some("2026-09-22".into()),
            project: Some("pi:atlas".into()),
        };
        assert_eq!(
            href(base, &sel),
            "/?tool=pi&bucket=daily&sel_bucket=2026-09-22&sel_project=pi%3Aatlas"
        );
    }
}
