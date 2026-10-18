//! Server-rendered page components.

use tokenburn_core::utils::fmt::thousands;
use tokenburn_core::{
    Bucket, BucketSavings, CURRENCIES, Currency, Metric, Query, ReportRow, Savings, Summary,
    ToolFilter, Window,
};
use topcoat::{
    Result,
    runtime::{Event, Signal},
    view::{View, component, view},
};

use crate::charts::{self, Bars, Col, SliceGeom};
use crate::data::{
    Dashboard, MAX_BUCKET_ROWS, bucket_noun, bucket_rows, savings_rows, share_percent,
};
use crate::params::WINDOW_CHOICES;
use crate::theme;
use tokenburn_core::Selection;

/// Title with the version, plus the Dashboard / Settings / About tabs.
#[component]
pub async fn brand(active: String) -> Result<impl View> {
    let pick = |name: &str| if active == name { "tab active" } else { "tab" };
    let (c_dash, c_set, c_about) = (pick("dashboard"), pick("settings"), pick("about"));
    Ok(view! {
        <div class="brand">
            <h1><a href="/" style="color:inherit">"🔥 TokenBurn"</a> <small class="ver">(tokenburn_core::appinfo::version_label())</small></h1>
            <nav class="tabs" aria-label="Pages">
                <a class=(c_dash) href="/">"Dashboard"</a>
                <a class=(c_set) href="/settings">"⚙ Settings"</a>
                <a class=(c_about) href="/about">"ℹ About"</a>
            </nav>
        </div>
    })
}

/// Progress of each spending limit (daily / weekly / monthly), coloured by how close
/// it is. Renders nothing when no limit is set.
#[component]
pub async fn limits_panel(
    check: &tokenburn_core::LimitCheck,
    currency: Currency,
) -> Result<impl View> {
    let rows: Vec<(String, String, u32, &'static str)> = check
        .statuses
        .iter()
        .map(|s| {
            (
                format!(
                    "{} · {} of {} ({}%)",
                    s.period.label(),
                    currency.format(s.spent_usd),
                    currency.format(s.limit_usd),
                    s.percent()
                ),
                match s.level {
                    tokenburn_core::Level::Ok => String::new(),
                    tokenburn_core::Level::Warn => "⚠ nearly reached".to_string(),
                    tokenburn_core::Level::Over => "🚨 exceeded".to_string(),
                },
                crate::data::bar_percent(s),
                crate::data::level_class(s.level),
            )
        })
        .collect();
    Ok(view! {
        if !rows.is_empty() {
            <section class="limits">
                for (label, note, pct, class) in &rows {
                    <div class=(*class)>
                        <div class="label">(label.clone()) " " <b>(note.clone())</b></div>
                        <progress max="100" value=(pct.to_string())>(pct.to_string()) "%"</progress>
                    </div>
                }
            </section>
        }
    })
}

/// Row of headline totals.
#[component]
pub async fn summary_cards(
    metric: Metric,
    total: Summary,
    currency: Currency,
    savings: &Savings,
) -> Result<impl View> {
    // Which figures show follows the chart metric (tokens / cost / saved).
    let cards = tokenburn_core::cards(metric, &total, savings, &currency);
    Ok(view! {
        <section class="cards">
            for c in &cards {
                <div class=(card_class(c.kind))><div class="label">(c.label.clone())</div><div class="value">(c.value.clone())</div></div>
            }
        </section>
    })
}

/// CSS class of a card: `card`, `card cost` or `card saved`.
pub fn card_class(kind: tokenburn_core::CardKind) -> &'static str {
    match kind {
        tokenburn_core::CardKind::Plain => "card",
        tokenburn_core::CardKind::Cost => "card cost",
        tokenburn_core::CardKind::Saved => "card saved",
    }
}

/// The bar chart's title for `metric`.
pub fn chart_title(metric: Metric) -> &'static str {
    match metric {
        Metric::Tokens => "Tokens over time",
        Metric::Cost => "Cost over time",
        Metric::Saved => "Saved over time (cache + compaction)",
    }
}

/// What prompt caching, rtk and pi compaction saved, per source. Renders nothing
/// when nothing was saved.
#[component]
pub async fn savings_panel(
    savings: &Savings,
    currency: Currency,
    spent_usd: f64,
    input_price: f64,
    buckets: &[BucketSavings],
    noun: &str,
) -> Result<impl View> {
    let rows = savings_rows(savings, currency, spent_usd);
    let per_bucket = bucket_rows(buckets, currency, MAX_BUCKET_ROWS);
    let per_title = format!("Saved per {noun}");
    let basis = format!(
        "estimate · valued at {}/M input tokens",
        currency.format(input_price)
    );
    Ok(view! {
        if !rows.is_empty() {
            <section class="savings">
                <h2>"Saved " <small class="ver">(basis)</small></h2>
                <div class="table-wrap"><table>
                    <thead>
                        <tr>
                            <th class="left">"source"</th>
                            <th>"tokens"</th>
                            <th>"saved"</th>
                            <th class="left opt2">"basis"</th>
                        </tr>
                    </thead>
                    <tbody>
                        for r in &rows {
                            <tr class=(if r.total { "total" } else { "" })>
                                <td class="left">(r.label.clone())</td>
                                <td>(r.tokens.clone())</td>
                                <td class="cost">(r.money.clone())</td>
                                <td class="left opt2">(r.basis.clone())</td>
                            </tr>
                        }
                    </tbody>
                </table></div>
                if !per_bucket.is_empty() {
                    <h2>(per_title)</h2>
                    <div class="table-wrap"><table class="per-bucket">
                        <thead>
                            <tr>
                                <th class="left">"interval"</th>
                                <th class="opt2">"tokens"</th>
                                <th>"saved"</th>
                                <th class="left opt2">"by source"</th>
                            </tr>
                        </thead>
                        <tbody>
                            for b in &per_bucket {
                                <tr>
                                    <td class="left">(b.label.clone())</td>
                                    <td class="opt2">(b.tokens.clone())</td>
                                    <td class="cost">(b.money.clone())</td>
                                    <td class="left opt2">(b.split.clone())</td>
                                </tr>
                            }
                        </tbody>
                    </table></div>
                }
            </section>
        }
    })
}

/// GET form that drives the query string.
#[component]
pub async fn filters(
    query: Query,
    theme: usize,
    metric: Metric,
    currency: Currency,
) -> Result<impl View> {
    Ok(view! {
        <form class="filters" method="get" action="/">
            <label>"chart "
                <select name="metric" onchange="this.form.submit()">
                    for m in Metric::ALL {
                        <option value=(m.label()) selected=(m == metric)>(m.label())</option>
                    }
                </select>
            </label>
            <label>"tool "
                <select name="tool" onchange="this.form.submit()">
                    for t in ToolFilter::ALL {
                        <option value=(t.as_str()) selected=(t == query.tool)>(t.as_str())</option>
                    }
                </select>
            </label>
            <label>"view "
                <select name="bucket" onchange="this.form.submit()">
                    for b in Bucket::ALL {
                        <option value=(b.label()) selected=(b == query.bucket)>(b.label())</option>
                    }
                </select>
            </label>
            <label>"window "
                <select name="window" onchange="this.form.submit()">
                    for w in Window::ALL {
                        <option value=(w.label()) selected=(w == query.window)>(w.label())</option>
                    }
                </select>
            </label>

            <label>"currency "
                <select name="currency" onchange="this.form.submit()">
                    for cur in CURRENCIES {
                        <option value=(cur.code) selected=(cur.code == currency.code)>(format!("{} · {}", cur.code, cur.name))</option>
                    }
                </select>
            </label>
            <label>"theme "
                <select name="theme" onchange="this.form.submit()">
                    for n in theme::names() {
                        <option value=(n) selected=(n == theme::name(theme))>(n)</option>
                    }
                </select>
            </label>
            <noscript><button type="submit">"Apply"</button></noscript>
        </form>
    })
}

/// Aggregated report table with a share-of-tokens bar per line.
#[component]
pub async fn report_table(report: &[ReportRow], currency: Currency) -> Result<impl View> {
    let bucketed = report.iter().any(|r| r.bucket.is_some());
    let max = report
        .iter()
        .filter(|r| r.tool != "TOTAL")
        .map(|r| r.summary.total_tokens)
        .max()
        .unwrap_or(0);

    Ok(view! {
        <div class="table-wrap"><table>
            <thead>
                <tr>
                    if bucketed { <th>"bucket"</th> }
                    <th class="left">"tool"</th>
                    <th class="opt2">"turns"</th>
                    <th class="opt2">"input"</th>
                    <th class="opt2">"output"</th>
                    <th class="opt1">"cache read"</th>
                    <th class="opt1">"cache write"</th>
                    <th>"total"</th>
                    <th class="left opt2">"share"</th>
                    <th>"cost"</th>
                </tr>
            </thead>
            <tbody>
                for r in report {
                    <tr class=(if r.tool == "TOTAL" { "total" } else { "" })>
                        if bucketed { <td>(r.bucket.clone().unwrap_or_default())</td> }
                        <td class="left">(r.tool.clone())</td>
                        <td class="opt2">(thousands(r.summary.turns))</td>
                        <td class="opt2">(thousands(r.summary.input))</td>
                        <td class="opt2">(thousands(r.summary.output))</td>
                        <td class="opt1">(thousands(r.summary.cache_read))</td>
                        <td class="opt1">(thousands(r.summary.cache_write))</td>
                        <td>(thousands(r.summary.total_tokens))</td>
                        <td class="left opt2">
                            if r.tool != "TOTAL" {
                                <span class="bar"><span style=(format!("width:{}%;background:{}", share_percent(r.summary.total_tokens, max), theme::series_var(&r.tool)))></span></span>
                            }
                        </td>
                        <td class="cost">(currency.format(r.summary.cost_usd))</td>
                    </tr>
                }
            </tbody>
        </table></div>
    })
}

/// Per-project / per-model breakdown, most expensive first.
#[component]
pub async fn projects_table(
    projects: &[(String, Summary)],
    currency: Currency,
) -> Result<impl View> {
    let max = projects
        .iter()
        .map(|(_, s)| (s.cost_usd * 10_000.0) as u64)
        .max()
        .unwrap_or(0);

    Ok(view! {
        <div class="table-wrap"><table>
            <thead>
                <tr>
                    <th class="left">"tool:project/model"</th>
                    <th class="opt2">"turns"</th>
                    <th>"total tokens"</th>
                    <th class="left opt2">"share"</th>
                    <th>"cost"</th>
                </tr>
            </thead>
            <tbody>
                for (name, s) in projects {
                    <tr>
                        <td class="left">(name.clone())</td>
                        <td class="opt2">(thousands(s.turns))</td>
                        <td>(thousands(s.total_tokens))</td>
                        <td class="left opt2">
                            <span class="bar"><span style=(format!("width:{}%;background:{}", share_percent((s.cost_usd * 10_000.0) as u64, max), theme::series_var(name.split(':').next().unwrap_or(""))))></span></span>
                        </td>
                        <td class="cost">(currency.format(s.cost_usd))</td>
                    </tr>
                }
            </tbody>
        </table></div>
    })
}

/// Selects bound to signals: changing one updates the signal in the browser
/// and (because the enclosing shard reads it) re-renders the shard.
#[component]
pub async fn live_filters(
    tool: &Signal<String>,
    bucket: &Signal<String>,
    window: &Signal<String>,
    theme: &Signal<String>,
    metric: &Signal<String>,
    currency: &Signal<String>,
) -> Result<impl View> {
    let cur = currency.get_untracked();
    let (t, b, w, th, m) = (
        tool.get_untracked(),
        bucket.get_untracked(),
        window.get_untracked(),
        theme.get_untracked(),
        metric.get_untracked(),
    );
    Ok(view! {
        <div class="filters">
            <label>"chart "
                <select :value=$(metric.get()) @change=$(|e: Event| metric.set(e.target.value))>
                    for x in Metric::ALL {
                        <option value=(x.label()) selected=(x.label() == m)>(x.label())</option>
                    }
                </select>
            </label>
            <label>"tool "
                <select :value=$(tool.get()) @change=$(|e: Event| tool.set(e.target.value))>
                    for x in ToolFilter::ALL {
                        <option value=(x.as_str()) selected=(x.as_str() == t)>(x.as_str())</option>
                    }
                </select>
            </label>
            <label>"view "
                <select :value=$(bucket.get()) @change=$(|e: Event| bucket.set(e.target.value))>
                    for x in Bucket::ALL {
                        <option value=(x.label()) selected=(x.label() == b)>(x.label())</option>
                    }
                </select>
            </label>
            <label>"window "
                <select :value=$(window.get()) @change=$(|e: Event| window.set(e.target.value))>
                    for x in WINDOW_CHOICES {
                        <option value=(x) selected=(x == w)>(x)</option>
                    }
                </select>
            </label>

            <label>"currency "
                <select :value=$(currency.get()) @change=$(|e: Event| currency.set(e.target.value))>
                    for cc in CURRENCIES {
                        <option value=(cc.code) selected=(cc.code == cur)>(format!("{} · {}", cc.code, cc.name))</option>
                    }
                </select>
            </label>
            <label>"theme "
                <select :value=$(theme.get()) @change=$(|e: Event| theme.set(e.target.value))>
                    for n in theme::names() {
                        <option value=(n) selected=(n == th)>(n)</option>
                    }
                </select>
            </label>
        </div>
    })
}

/// Cards, report and project tables for one [`Dashboard`] snapshot.
///
/// `live` shows the "live" badge (the page has an open connection and is
/// refreshing itself).
#[component]
pub async fn dashboard_body(
    d: &Dashboard,
    live: bool,
    metric: Metric,
    base: &str,
    click_bucket: Option<&Signal<String>>,
    click_project: Option<&Signal<String>>,
) -> Result<impl View> {
    Ok(view! {
        for w in &d.warnings {
            <div class="warn">"⚠ " (w.clone())</div>
        }

        limits_panel(check: &d.limits, currency: d.currency)

        summary_cards(metric: metric, total: d.total, currency: d.currency, savings: &d.savings)


        charts_panel(d: d, metric: metric, base: base, animate: live, click_bucket: click_bucket, click_project: click_project)

        if metric == Metric::Saved {
            savings_panel(savings: &d.savings, currency: d.currency, spent_usd: d.total.cost_usd, input_price: d.input_price, buckets: &d.savings_by_bucket, noun: bucket_noun(&d.query))
        }

        <section>
            <h2>"Report · " (d.query.bucket.label())</h2>
            if d.report.is_empty() {
                <div class="empty">
                    "No tokenburn results found for tool=" (d.query.tool.as_str())
                    ", window=" (d.query.window.label()) "."
                </div>
            } else {
                report_table(report: &d.report, currency: d.currency)
            }
        </section>

        if tokenburn_core::projects_add_detail(&d.report, &d.projects) {
            <section>
                <h2>"Projects / models (by cost)"</h2>
                projects_table(projects: &d.projects, currency: d.currency)
            </section>
        }

        <footer>
            if live {
                <span class="badge live">"● live"</span>
            } else {
                <span class="badge">"○ static"</span>
            }
            " "
            (d.row_count) " rows · updated " (d.updated_at.clone())
            " · local logs only · JSON at "
            <a href="/api/report">"/api/report"</a>
        </footer>
    })
}

/// The active theme's CSS variables, as a `<style>` element.
#[component]
pub async fn theme_style(index: usize) -> Result<impl View> {
    let css = theme::css(index);
    Ok(view! { <style>(css)</style> })
}

/// What a click on a bar or slice does.
///
/// * **Live** (runtime available): the handler sets the page's selection signals; the
///   shard re-renders in place, so CSS transitions animate the change.
/// * **Static**: every bar and slice is a link to the same page with the new
///   selection in the query string (`base` is that page's URL without it).
#[derive(Clone, Copy)]
pub struct Clicks<'a> {
    pub bucket: Option<&'a Signal<String>>,
    pub project: Option<&'a Signal<String>>,
}

/// One bar: native tooltip, full-height hit area, the stacked segments and a
/// selection outline. The wrapper (`<g>` + click handler/link) is added by the caller.
#[component]
pub async fn bar_col(col: &Col) -> Result<impl View> {
    Ok(view! {
        <title>(col.title.clone())</title>
        <rect class="hit" x=(col.slot_x.clone()) y=(col.plot_y.clone()) width=(col.slot_w.clone()) height=(col.plot_h.clone())></rect>
        for seg in &col.segs {
            <rect class="seg" x=(seg.x.clone()) y=(seg.y.clone()) width=(seg.w.clone()) height=(seg.h.clone()) fill=(seg.var.clone())></rect>
        }
        <rect class="outline" x=(col.slot_x.clone()) y=(col.plot_y.clone()) width=(col.slot_w.clone()) height=(col.plot_h.clone())></rect>
    })
}

/// One donut slice (the ring segment with its tooltip).
#[component]
pub async fn slice_path(slice: &SliceGeom) -> Result<impl View> {
    Ok(view! {
        <path d=(slice.path.clone()) fill=(slice.var.clone())>
            <title>(slice.title.clone())</title>
        </path>
    })
}

fn col_class(c: &Col) -> String {
    if c.selected {
        "col sel".into()
    } else {
        "col".into()
    }
}

fn slice_class(s: &SliceGeom) -> String {
    let mut c = String::from("slice");
    if s.selected {
        c.push_str(" sel");
    }
    if s.key.is_none() {
        c.push_str(" other");
    }
    c
}

/// Stagger so the whole sweep takes ~0.6 s however many bars there are.
fn bar_delay(index: usize, n: usize) -> String {
    let step = (600.0 / n.max(1) as f32).min(14.0);
    format!("--d:{:.0}ms", step * index as f32)
}

/// The bar chart (`<svg>`).
#[component]
pub async fn bars_chart(
    bars: &Bars,
    metric: Metric,
    base: &str,
    sel: &Selection,
    animate: bool,
    click: Option<&Signal<String>>,
) -> Result<impl View> {
    let n = bars.cols.len();
    // The selection each column would produce when clicked (toggle).
    let cols: Vec<(Col, String, String, String)> = bars
        .cols
        .iter()
        .map(|c| {
            let mut next = sel.clone();
            next.toggle_bucket(&c.label);
            (
                c.clone(),
                charts::href(base, &next),
                next.bucket.clone().unwrap_or_default(),
                format!("{} {}", col_class(c), if animate { "anim" } else { "" }),
            )
        })
        .collect();
    let class = format!(
        "chart bars{}{}",
        if bars.has_selection { " has-sel" } else { "" },
        if animate { " animate" } else { "" }
    );
    let label = format!("{} over time", metric.label());

    Ok(view! {
        <svg class=(class) viewBox=(format!("0 0 {} {}", charts::BARS_W, charts::BARS_H)) width="100%" role="img" aria-label=(label)>
            if bars.empty {
                <text class="empty-note" x=(charts::BARS_W / 2.0) y=(charts::BARS_H / 2.0) text-anchor="middle">"No data in this window"</text>
            }
            for g in &bars.grid {
                <line class="grid" x1=(bars.left.clone()) x2=(bars.right.clone()) y1=(g.y.clone()) y2=(g.y.clone())></line>
                <text class="axis" x=(charts::LEFT - 6.0) y=(g.text_y.clone()) text-anchor="end">(g.label.clone())</text>
            }
            for (c, href, next, class) in cols {
                if let Some(sig) = click {
                    <g class=(class) style=(bar_delay(c.index, n)) @click=$(|_e| sig.set(next))>
                        bar_col(col: &c)
                    </g>
                } else {
                    <a href=(href)>
                        <g class=(class) style=(bar_delay(c.index, n))>
                            bar_col(col: &c)
                        </g>
                    </a>
                }
            }
            for l in &bars.x_labels {
                <text class=(if l.alt { "axis alt" } else { "axis" }) x=(l.x.clone()) y=(l.y.clone()) text-anchor="middle">(l.text.clone())</text>
            }
        </svg>
    })
}

/// The share donut (`<svg>`), its slices sweeping in through a mask.
#[component]
pub async fn donut_chart(
    donut: &charts::Donut,
    base: &str,
    sel: &Selection,
    animate: bool,
    click: Option<&Signal<String>>,
) -> Result<impl View> {
    let slices: Vec<(SliceGeom, String, String, String)> = donut
        .slices
        .iter()
        .map(|s| {
            let mut next = sel.clone();
            if let Some(k) = &s.key {
                next.toggle_project(k);
            }
            (
                s.clone(),
                charts::href(base, &next),
                next.project.clone().unwrap_or_default(),
                format!("--dx:{};--dy:{}", s.dx, s.dy),
            )
        })
        .collect();
    let class = format!(
        "chart donut{}{}",
        if donut.has_selection { " has-sel" } else { "" },
        if animate { " animate" } else { "" }
    );
    let total = donut.total_label.clone();

    Ok(view! {
        <svg class=(class) viewBox="0 0 200 200" width="100%" role="img" aria-label="share by project">
            if donut.empty {
                <text class="empty-note" x="100" y="104" text-anchor="middle">"No data"</text>
            } else {
                <defs>
                    <mask id="sweep" maskUnits="userSpaceOnUse" x="0" y="0" width="200" height="200">
                        <circle class="sweep" cx="100" cy="100" r="74" fill="none" stroke="white" stroke-width="60" pathLength="1" transform="rotate(-90 100 100)"></circle>
                    </mask>
                </defs>
                <g mask="url(#sweep)">
                    for (s, href, next, vars) in slices {
                        if s.key.is_none() {
                            <g class=(slice_class(&s)) style=(vars)>
                                slice_path(slice: &s)
                            </g>
                        } else if let Some(sig) = click {
                            <g class=(slice_class(&s)) style=(vars) @click=$(|_e| sig.set(next))>
                                slice_path(slice: &s)
                            </g>
                        } else {
                            <a href=(href)>
                                <g class=(slice_class(&s)) style=(vars)>
                                    slice_path(slice: &s)
                                </g>
                            </a>
                        }
                    }
                </g>
                <text class="total" x="100" y="106" text-anchor="middle">(total)</text>
            }
        </svg>
    })
}

/// "Showing …" chips with a clear control, or a hint when nothing is selected.
#[component]
pub async fn selection_bar(
    sel: &Selection,
    base: &str,
    click_bucket: Option<&Signal<String>>,
    click_project: Option<&Signal<String>>,
) -> Result<impl View> {
    let chips = sel.describe();
    let clear = charts::href(base, &Selection::default());
    // Runtime expressions only take single-identifier paths, so the empty values are passed in.
    let (blank_a, blank_b) = (String::new(), String::new());
    Ok(view! {
        <div class="selbar">
            if chips.is_empty() {
                <span class="hint">"click a bar or a slice to drill down"</span>
            } else {
                <span class="hint">"Showing"</span>
                for chip in chips {
                    <span class="chip">(chip)</span>
                }
                if let (Some(b), Some(p)) = (click_bucket, click_project) {
                    <button class="clear" @click=$(|_e| { b.set(blank_a); p.set(blank_b); })>"✕ clear"</button>
                } else {
                    <a class="clear" href=(clear)>"✕ clear"</a>
                }
            }
        </div>
    })
}

/// Stacked bars over time and the share donut, both clickable.
#[component]
pub async fn charts_panel(
    d: &Dashboard,
    metric: Metric,
    base: &str,
    animate: bool,
    click_bucket: Option<&Signal<String>>,
    click_project: Option<&Signal<String>>,
) -> Result<impl View> {
    let cur = d.currency;
    let bars = charts::bars(
        &d.series.in_currency(&cur),
        metric,
        d.selection.bucket.as_deref(),
        &cur,
    );
    let donut = charts::donut(
        &d.pie.in_currency(&cur),
        d.selection.project.as_deref(),
        &cur,
    );
    let sel = d.selection.clone();
    let pie_metric = d.pie.metric;

    // Legend rows (clickable like the slices): swatch, label, share.
    let legend: Vec<(SliceGeom, String, String)> = donut
        .slices
        .iter()
        .map(|s| {
            let mut next = sel.clone();
            if let Some(k) = &s.key {
                next.toggle_project(k);
            }
            (
                s.clone(),
                charts::href(base, &next),
                next.project.clone().unwrap_or_default(),
            )
        })
        .collect();
    let swatches: Vec<(String, String)> = d
        .series
        .active_tools(metric)
        .into_iter()
        .map(|t| (charts::tool_var(t), t.display_name().to_string()))
        .collect();

    Ok(view! {
        <section class="charts">
            <div class="panel">
                <h2>(chart_title(metric))</h2>
                bars_chart(bars: &bars, metric: metric, base: base, sel: &sel, animate: animate, click: click_bucket)
                <ul class="legend">
                    for (var, name) in swatches {
                        <li><span class="sw" style=(format!("background:{var}"))></span>(name)</li>
                    }
                </ul>
            </div>
            <div class="panel">
                <h2>"Share by project · " (pie_metric.label())</h2>
                <div class="donut-row">
                    <div class="donut-box">
                        donut_chart(donut: &donut, base: base, sel: &sel, animate: animate, click: click_project)
                    </div>
                    <ul class=(if donut.has_selection { "legend vertical has-sel" } else { "legend vertical" })>
                        for (s, href, next) in legend {
                            if s.key.is_none() {
                                <li class="item other"><span class="sw" style=(format!("background:{}", s.var))></span>(s.label.clone()) <span class="pct">(s.pct.clone())</span></li>
                            } else if let Some(sig) = click_project {
                                <li class=(if s.selected { "item sel" } else { "item" }) @click=$(|_e| sig.set(next))>
                                    <span class="sw" style=(format!("background:{}", s.var))></span>(s.label.clone()) <span class="pct">(s.pct.clone())</span>
                                </li>
                            } else {
                                <li class=(if s.selected { "item sel" } else { "item" })>
                                    <a href=(href)><span class="sw" style=(format!("background:{}", s.var))></span>(s.label.clone()) <span class="pct">(s.pct.clone())</span></a>
                                </li>
                            }
                        }
                    </ul>
                </div>
            </div>
        </section>
        selection_bar(sel: &sel, base: base, click_bucket: click_bucket, click_project: click_project)
    })
}
