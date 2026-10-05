//! Server-rendered page components.

use tokenburn_core::utils::fmt::{compact, thousands, usd};
use tokenburn_core::{Bucket, Query, ReportRow, Summary, ToolFilter, Window};
use topcoat::{
    Result,
    runtime::{Event, Signal},
    view::{View, component, view},
};

use crate::data::{Dashboard, share_percent};
use crate::params::WINDOW_CHOICES;

/// Row of headline totals.
#[component]
pub async fn summary_cards(total: Summary) -> Result<impl View> {
    Ok(view! {
        <section class="cards">
            <div class="card"><div class="label">"turns"</div><div class="value">(thousands(total.turns))</div></div>
            <div class="card"><div class="label">"total tokens"</div><div class="value">(compact(total.total_tokens))</div></div>
            <div class="card"><div class="label">"input"</div><div class="value">(compact(total.input))</div></div>
            <div class="card"><div class="label">"output"</div><div class="value">(compact(total.output))</div></div>
            <div class="card"><div class="label">"cache read"</div><div class="value">(compact(total.cache_read))</div></div>
            <div class="card"><div class="label">"cache write"</div><div class="value">(compact(total.cache_write))</div></div>
            <div class="card cost"><div class="label">"cost"</div><div class="value">(usd(total.cost_usd))</div></div>
        </section>
    })
}

/// GET form that drives the query string.
#[component]
pub async fn filters(query: Query) -> Result<impl View> {
    Ok(view! {
        <form class="filters" method="get" action="/">
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
            <button type="submit">"Refresh"</button>
        </form>
    })
}

/// Aggregated report table with a share-of-tokens bar per line.
#[component]
pub async fn report_table(report: &[ReportRow]) -> Result<impl View> {
    let bucketed = report.iter().any(|r| r.bucket.is_some());
    let max = report
        .iter()
        .filter(|r| r.tool != "TOTAL")
        .map(|r| r.summary.total_tokens)
        .max()
        .unwrap_or(0);

    Ok(view! {
        <table>
            <thead>
                <tr>
                    if bucketed { <th>"bucket"</th> }
                    <th class="left">"tool"</th>
                    <th>"turns"</th>
                    <th>"input"</th>
                    <th>"output"</th>
                    <th>"cache read"</th>
                    <th>"cache write"</th>
                    <th>"total"</th>
                    <th>"cost"</th>
                    <th class="left">"share"</th>
                </tr>
            </thead>
            <tbody>
                for r in report {
                    <tr class=(if r.tool == "TOTAL" { "total" } else { "" })>
                        if bucketed { <td>(r.bucket.clone().unwrap_or_default())</td> }
                        <td class="left">(r.tool.clone())</td>
                        <td>(thousands(r.summary.turns))</td>
                        <td>(thousands(r.summary.input))</td>
                        <td>(thousands(r.summary.output))</td>
                        <td>(thousands(r.summary.cache_read))</td>
                        <td>(thousands(r.summary.cache_write))</td>
                        <td>(thousands(r.summary.total_tokens))</td>
                        <td class="cost">(usd(r.summary.cost_usd))</td>
                        <td class="left">
                            if r.tool != "TOTAL" {
                                <span class="bar"><span style=(format!("width:{}%", share_percent(r.summary.total_tokens, max)))></span></span>
                            }
                        </td>
                    </tr>
                }
            </tbody>
        </table>
    })
}

/// Per-project (pi) / per-model (Zed) breakdown, most expensive first.
#[component]
pub async fn projects_table(projects: &[(String, Summary)]) -> Result<impl View> {
    let max = projects
        .iter()
        .map(|(_, s)| (s.cost_usd * 10_000.0) as u64)
        .max()
        .unwrap_or(0);

    Ok(view! {
        <table>
            <thead>
                <tr>
                    <th class="left">"tool:project/model"</th>
                    <th>"turns"</th>
                    <th>"total tokens"</th>
                    <th>"cost"</th>
                    <th class="left">"share"</th>
                </tr>
            </thead>
            <tbody>
                for (name, s) in projects {
                    <tr>
                        <td class="left">(name.clone())</td>
                        <td>(thousands(s.turns))</td>
                        <td>(thousands(s.total_tokens))</td>
                        <td class="cost">(usd(s.cost_usd))</td>
                        <td class="left">
                            <span class="bar"><span style=(format!("width:{}%", share_percent((s.cost_usd * 10_000.0) as u64, max)))></span></span>
                        </td>
                    </tr>
                }
            </tbody>
        </table>
    })
}

/// Selects bound to signals: changing one updates the signal in the browser
/// and (because the enclosing shard reads it) re-renders the shard.
#[component]
pub async fn live_filters(
    tool: &Signal<String>,
    bucket: &Signal<String>,
    window: &Signal<String>,
) -> Result<impl View> {
    let (t, b, w) = (
        tool.get_untracked(),
        bucket.get_untracked(),
        window.get_untracked(),
    );
    Ok(view! {
        <div class="filters">
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
        </div>
    })
}

/// Cards, report and project tables for one [`Dashboard`] snapshot.
///
/// `live` shows the "live" badge (the page has an open connection and is
/// refreshing itself).
#[component]
pub async fn dashboard_body(d: &Dashboard, live: bool) -> Result<impl View> {
    Ok(view! {
        for w in &d.warnings {
            <div class="warn">"⚠ " (w.clone())</div>
        }

        summary_cards(total: d.total)

        <section>
            <h2>"Report · " (d.query.bucket.label())</h2>
            if d.report.is_empty() {
                <div class="empty">
                    "No tokenburn results found for tool=" (d.query.tool.as_str())
                    ", window=" (d.query.window.label()) "."
                </div>
            } else {
                report_table(report: &d.report)
            }
        </section>

        if !d.projects.is_empty() {
            <section>
                <h2>"Projects / models (by cost)"</h2>
                projects_table(projects: &d.projects)
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
