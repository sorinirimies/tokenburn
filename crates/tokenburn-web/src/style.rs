//! Stylesheet served at `/style.css`.
//!
//! Responsive on the same breakpoints as the TUI and the GUI (`LayoutClass` in
//! `tokenburn-core`): **wide** ≥ 980 px · **medium** ≥ 700 px · **narrow** below.
//! Medium stacks the charts and drops the cache columns; narrow also compacts the
//! header and controls, keeps only the essential columns and enlarges the chart text.

pub const CSS: &str = r#"
:root { color-scheme: dark; --bg:#14161f; --panel:#1c1f2b; --line:#2b3042; --fg:#d8dcea; --dim:#8b92aa; --accent:#7aa2f7; --ok:#9ece6a; --warn:#e0af68; --err:#f7768e; --fg2:#e0af68; --sel:#232840; --s0:#7aa2f7; --s1:#bb9af7; --s2:#e0af68; --s3:#9ece6a; --s4:#f7768e; --s5:#7dcfff; --s6:#ff9e64; --s7:#a9b1d6; }
* { box-sizing: border-box; }
body { margin:0; background:var(--bg); color:var(--fg); font:14px/1.5 system-ui,-apple-system,"Segoe UI",sans-serif; }
main { max-width:1200px; margin:0 auto; padding:24px; display:flex; flex-direction:column; gap:20px; }
header.top { display:flex; flex-wrap:wrap; align-items:center; gap:16px; justify-content:space-between; }
h1 { margin:0; font-size:22px; color:var(--accent); }
h2 { margin:0 0 8px; font-size:14px; color:var(--dim); text-transform:uppercase; letter-spacing:.06em; }
form.filters { display:flex; flex-wrap:wrap; gap:10px; align-items:center; }
select, button { background:var(--panel); color:var(--fg); border:1px solid var(--line); border-radius:6px; padding:6px 10px; font:inherit; }
button { cursor:pointer; border-color:var(--accent); }
.cards { display:grid; grid-template-columns:repeat(auto-fit,minmax(150px,1fr)); gap:10px; }
.card { background:var(--panel); border:1px solid var(--line); border-radius:8px; padding:12px; }
.card .label { color:var(--dim); font-size:12px; }
.card .value { font-size:22px; font-variant-numeric:tabular-nums; }
.card.cost .value { color:var(--ok); }
table { width:100%; border-collapse:collapse; background:var(--panel); border:1px solid var(--line); border-radius:8px; overflow:hidden; }
th, td { padding:8px 12px; text-align:right; font-variant-numeric:tabular-nums; border-bottom:1px solid var(--line); }
th { color:var(--accent); font-weight:600; background:var(--sel); }
th:first-child, td:first-child, td.left, th.left { text-align:left; }
tr.total td { color:var(--warn); font-weight:700; }
td.cost { color:var(--ok); }
.bar { width:140px; height:8px; background:var(--line); border-radius:4px; overflow:hidden; display:inline-block; }
.bar > span { display:block; height:100%; background:var(--accent); }
tr:hover td { background:var(--sel); }
.empty, .warn { padding:16px; background:var(--panel); border:1px solid var(--line); border-radius:8px; color:var(--dim); }
.warn { color:var(--warn); }
.badge { padding:2px 8px; border-radius:10px; border:1px solid var(--line); }
.badge.live { color:var(--ok); border-color:var(--ok); }
.filters { display:flex; flex-wrap:wrap; gap:10px; align-items:center; }
.charts { display:grid; grid-template-columns:3fr 2fr; gap:16px; }
.panel { background:var(--panel); border:1px solid var(--line); border-radius:8px; padding:12px 14px; }
.panel h2 { margin:0 0 6px; }
svg.chart { display:block; max-width:100%; height:auto; overflow:visible; }
svg.chart text { font-family:inherit; }
svg.chart text.axis { fill:var(--dim); font-size:12px; }
svg.chart text.total { fill:var(--fg); font-size:17px; }
svg.chart text.empty-note { fill:var(--dim); font-size:14px; }
svg.chart line.grid { stroke:var(--line); stroke-width:1; }
svg.chart a { cursor:pointer; }

/* ── bars: click to select a bucket ── */
.bars .col { cursor:pointer; transition:opacity .2s ease; }
.bars .col .hit { fill:transparent; }
.bars .col .seg { transition:y .45s ease, height .45s ease, filter .15s ease; }
.bars .col .outline { fill:none; stroke:var(--accent); stroke-width:2; opacity:0; pointer-events:none; transition:opacity .15s ease; }
.bars .col:hover .seg { filter:brightness(1.2); }
.bars .col:hover .outline { opacity:.45; }
.bars .col.sel .outline { opacity:1; }
.bars.has-sel .col:not(.sel) { opacity:.35; }
.bars.has-sel .col:not(.sel):hover { opacity:.75; }

/* ── donut: click to select a project ── */
.donut .slice { cursor:pointer; transition:transform .22s cubic-bezier(.2,.8,.3,1), opacity .2s ease; }
.donut .slice.other { cursor:default; }
.donut .slice:not(.other):hover, .donut .slice.sel { transform:translate(calc(var(--dx) * 1px), calc(var(--dy) * 1px)); }
.donut .slice path { transition:filter .15s ease; }
.donut .slice:not(.other):hover path { filter:brightness(1.15); }
.donut .slice.sel path { stroke:var(--accent); stroke-width:1.5; }
.donut.has-sel .slice:not(.sel) { opacity:.35; }

/* ── entry animation: bars grow up from the axis, the ring sweeps round ── */
@keyframes bar-grow { from { transform:scaleY(0); } to { transform:scaleY(1); } }
@keyframes ring-sweep { from { stroke-dashoffset:1; } to { stroke-dashoffset:0; } }
@keyframes fade-in { from { opacity:0; } to { opacity:1; } }
.bars.animate .col.anim { transform-box:fill-box; transform-origin:50% 100%; animation:bar-grow .6s cubic-bezier(.2,.8,.25,1) both; animation-delay:var(--d, 0ms); }
.donut.animate .sweep { stroke-dasharray:1; stroke-dashoffset:1; animation:ring-sweep .9s cubic-bezier(.3,.7,.2,1) forwards; }
.donut.animate text.total { animation:fade-in .6s ease .3s both; }
@media (prefers-reduced-motion: reduce) {
  .bars.animate .col.anim, .donut.animate .sweep, .donut.animate text.total { animation:none; }
  .donut .sweep { stroke-dasharray:none; stroke-dashoffset:0; }
  .bars .col, .bars .col .seg, .donut .slice { transition:none; }
}
.donut .sweep { stroke-dasharray:1; stroke-dashoffset:0; }

/* ── selection bar and legends ── */
.selbar { display:flex; flex-wrap:wrap; align-items:center; gap:8px; margin:-8px 0 0; font-size:12px; color:var(--dim); min-height:26px; }
.selbar .chip { background:var(--sel); border:1px solid var(--line); border-radius:12px; padding:1px 10px; color:var(--fg); }
.selbar .clear { cursor:pointer; background:var(--panel); color:var(--fg); border:1px solid var(--accent); border-radius:6px; padding:1px 8px; font:inherit; text-decoration:none; }
.legend .item { cursor:pointer; padding:1px 6px; margin:0 -6px; border-radius:4px; transition:background .15s ease, opacity .2s ease; }
.legend .item:hover { background:var(--sel); }
.legend .item.sel { background:var(--sel); outline:1px solid var(--accent); }
.legend .item.other { cursor:default; }
.legend .item.other:hover { background:none; }
.legend .item a { color:inherit; text-decoration:none; display:block; }
.legend.has-sel .item:not(.sel) { opacity:.45; }
.legend { list-style:none; margin:6px 0 0; padding:0; display:flex; flex-wrap:wrap; gap:4px 14px; font-size:12px; color:var(--dim); }
.legend.vertical { flex-direction:column; flex-wrap:nowrap; margin:0; gap:6px; color:var(--fg); }
.legend .sw { display:inline-block; width:10px; height:10px; border-radius:2px; margin-right:6px; }
.legend .pct { color:var(--dim); margin-left:6px; }
.donut-row { display:flex; align-items:center; gap:16px; }
.donut-box { width:190px; flex:none; }
footer { color:var(--dim); font-size:12px; }
a { color:var(--accent); }

/* ── responsive ── */
.table-wrap { overflow-x:auto; border-radius:8px; }
.table-wrap table { min-width:100%; }
.donut-box svg { max-width:240px; margin:0 auto; }
html { -webkit-text-size-adjust:100%; }
.legend.vertical { min-width:0; }

/* medium: below 980 px — charts stack, cache columns drop out */
@media (max-width:979px) {
  .charts { grid-template-columns:1fr; }
  .opt1 { display:none; }
  .cards { grid-template-columns:repeat(4, minmax(0,1fr)); }
  .donut-row { justify-content:center; }
}

/* narrow: below 700 px — compact header and controls, essential columns only */
@media (max-width:699px) {
  main { padding:12px 10px; gap:14px; }
  header.top { flex-direction:column; align-items:stretch; gap:10px; }
  header.top h1 { font-size:20px; }
  form.filters, .filters { display:grid; grid-template-columns:repeat(2, minmax(0,1fr)); gap:8px 10px; }
  .filters label { display:flex; flex-direction:column; gap:2px; font-size:12px; color:var(--dim); min-width:0; }
  .filters select { width:100%; min-width:0; }
  .cards { grid-template-columns:repeat(2, minmax(0,1fr)); }
  .card .value { font-size:19px; }
  .card.cost { grid-column:1 / -1; }
  .opt2 { display:none; }
  th, td { padding:7px 8px; }
  .panel { padding:10px; }
  .donut-row { flex-direction:column; align-items:stretch; gap:8px; }
  .donut-box { width:min(220px, 70vw); margin:0 auto; }
  .legend.vertical { flex-direction:row; flex-wrap:wrap; gap:4px 12px; }
  .selbar { margin:0; }
  /* The SVG is scaled to the screen, so its text must be bigger and sparser. */
  svg.chart text.axis { font-size:22px; }
  svg.chart text.axis.alt { display:none; }
  svg.chart text.total { font-size:20px; }
  .bars .col .outline { stroke-width:3; }
  footer { word-break:break-word; }
}
"#;

#[cfg(test)]
mod tests {
    use super::CSS;
    use tokenburn_core::utils::layout::{MEDIUM_PX, WIDE_PX};

    #[test]
    fn the_media_queries_use_the_shared_breakpoints() {
        // max-width is inclusive, so "below WIDE" is WIDE - 1.
        assert!(CSS.contains(&format!("@media (max-width:{}px)", WIDE_PX as u32 - 1)));
        assert!(CSS.contains(&format!("@media (max-width:{}px)", MEDIUM_PX as u32 - 1)));
    }

    #[test]
    fn narrow_screens_hide_the_secondary_columns_and_enlarge_the_chart_text() {
        let narrow = CSS.split("@media (max-width:699px)").nth(1).unwrap();
        assert!(narrow.contains(".opt2 { display:none; }"));
        assert!(narrow.contains("svg.chart text.axis { font-size:22px; }"));
        assert!(narrow.contains("svg.chart text.axis.alt { display:none; }"));
        let medium = CSS.split("@media (max-width:979px)").nth(1).unwrap();
        assert!(
            medium.contains(".opt1 { display:none; }")
                && medium.contains(".charts { grid-template-columns:1fr; }")
        );
    }

    #[test]
    fn wide_screens_never_hide_anything() {
        // The hiding rules live only inside the media queries.
        let before_queries = CSS.split("/* medium: below 980 px").next().unwrap();
        assert!(!before_queries.contains(".opt1 { display:none"));
        assert!(!before_queries.contains(".opt2 { display:none"));
    }
}
