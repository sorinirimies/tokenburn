//! Pages: the document shell, the home page and the live dashboard shard.

use topcoat::{
    Result,
    context::Cx,
    router::{page, query_params},
    runtime::{connected, shard, signal},
    view::{Child, View, component, emit, live, view},
};

use crate::{
    app::is_interactive,
    components::{brand, dashboard_body, filters, live_filters, theme_style},
    data::{REFRESH_EVERY, apply_limit_params, load},
    params::{LimitParams, UsageParams, parse, parse_metric, parse_selection, pick_currency},
    theme,
};

use tokenburn_core::settings;

/// HTML shell shared by every page.
#[component]
pub async fn document(
    interactive: bool,
    theme: usize,
    title: String,
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                if !interactive {
                    // Static fallback: no runtime, so reload every 10 s (the live mode pushes updates).
                    <meta http-equiv="refresh" content="10">
                }
                <title>(title)</title>
                <link rel="stylesheet" href="/style.css">
                // The theme's CSS variables override the stylesheet's defaults.
                theme_style(index: theme)
                <script src="/lifecycle.js"></script>
                topcoat::dev::script()
                if interactive {
                    topcoat::runtime::script()
                }
            </head>
            <body><main>(child)</main></body>
        </html>
    })
}

/// The dashboard URL for `query`/`metric` *without* a selection — what the static
/// page's bar and slice links build on. The theme is left out on purpose: the saved
/// one applies, so following a link never re-asserts (and re-saves) a stale choice.
pub fn page_base(query: &tokenburn_core::Query, metric: tokenburn_core::Metric) -> String {
    format!(
        "/?tool={}&bucket={}&window={}&metric={}",
        query.tool.as_str(),
        query.bucket.label(),
        query.window.label(),
        metric.label(),
    )
}

/// The currency to show for an optional explicit code: a valid one wins and is saved
/// (shared with the TUI, GUI and CLI), anything else keeps the saved one.
fn effective_currency(requested: Option<&str>) -> tokenburn_core::Currency {
    let st = settings::load();
    let saved = tokenburn_core::features::currency::resolve(&st.currency, &st.rates);
    let currency = pick_currency(requested, saved, &st.rates);
    if currency.code != saved.code {
        let _ = settings::save_currency(currency.code);
    }
    currency
}

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    let params = query_params::<UsageParams>(cx)?;
    let interactive = is_interactive(cx);
    let metric = parse_metric(params.metric.as_deref());

    // An explicit, valid `?theme=` wins and is remembered (shared with the TUI and GUI).
    let saved = theme::saved_index();
    let theme_index = theme::resolve(params.theme.as_deref(), saved);
    if theme_index != saved {
        theme::save_index(theme_index);
    }

    // Likewise `?currency=`: explicit and valid wins and is remembered.
    let currency = effective_currency(params.currency.as_deref());

    // Static fallback loads once here; the live path loads inside the shard.
    let selection = params.selection();
    let snapshot = if interactive {
        None
    } else {
        Some(load(params.to_query(), &selection, currency).await)
    };
    let base = snapshot
        .as_ref()
        .map(|d| page_base(&d.query, metric))
        .unwrap_or_default();

    // Initial control values come from the URL; signals take over afterwards.
    let tool = params.tool.clone().unwrap_or_else(|| "all".into());
    let bucket = params.bucket.clone().unwrap_or_else(|| "total".into());
    let window = params.window.clone().unwrap_or_else(|| "auto".into());

    Ok(view! {
        document(interactive: interactive, theme: theme_index, title: format!("{} — AI token usage", tokenburn_core::appinfo::title()),
            if let Some(d) = &snapshot {
                <header class="top">
                    brand(active: "dashboard".to_string())
                    filters(query: d.query, theme: theme_index, metric: metric, currency: currency)
                </header>
                dashboard_body(d: d, live: false, metric: metric, base: &base, click_bucket: None, click_project: None)
            } else {
                <header class="top">brand(active: "dashboard".to_string())</header>
                live_dashboard(tool: tool, bucket: bucket, window: window, theme: theme::name(theme_index).to_string(), metric: metric.label().to_string(), currency: currency.code.to_string(), sel_bucket: selection.bucket.clone().unwrap_or_default(), sel_project: selection.project.clone().unwrap_or_default())
            }
        )
    })
}

/// The saved theme and currency, shared by the pages that are not the dashboard.
fn saved_theme_and_currency() -> (usize, tokenburn_core::Currency) {
    (theme::saved_index(), settings::currency())
}

#[page("/about")]
async fn about_page(cx: &Cx) -> Result<impl View> {
    let _ = cx;
    let (theme_index, _) = saved_theme_and_currency();
    let rows = tokenburn_core::appinfo::about_rows();
    Ok(view! {
        document(interactive: false, theme: theme_index, title: format!("About — {}", tokenburn_core::appinfo::title()),
            <header class="top">brand(active: "about".to_string())</header>
            <section>
                <h2>(tokenburn_core::appinfo::title())</h2>
                <p class="dim">(tokenburn_core::appinfo::TAGLINE)</p>
                <table class="kv"><tbody>
                    for (k, v) in rows {
                        <tr>
                            <th>(k)</th>
                            <td>
                                if tokenburn_core::appinfo::is_link(&v) {
                                    <a href=(v.clone()) target="_blank" rel="noopener">(v.clone())</a>
                                } else {
                                    (v.clone())
                                }
                            </td>
                        </tr>
                    }
                </tbody></table>
            </section>
        )
    })
}

#[page("/settings")]
async fn settings_page(cx: &Cx) -> Result<impl View> {
    let (theme_index, currency) = saved_theme_and_currency();
    // The limits form: a submit saves, a bad number says so and keeps the old limits.
    let submitted = query_params::<LimitParams>(cx)?;
    let mut limits = settings::load().limits;
    let mut message = String::new();
    if submitted.save.is_some() {
        match apply_limit_params(&limits, submitted) {
            Ok(new) => {
                let _ = settings::save_limits(&new);
                limits = new;
                message = "Limits saved.".to_string();
            }
            Err(e) => message = e,
        }
    }
    let field =
        |p: tokenburn_core::Period| limits.get(p).map(|v| v.to_string()).unwrap_or_default();
    let (daily, weekly, monthly) = (
        field(tokenburn_core::Period::Day),
        field(tokenburn_core::Period::Week),
        field(tokenburn_core::Period::Month),
    );
    let notify = limits.notify;
    Ok(view! {
        document(interactive: false, theme: theme_index, title: format!("Settings — {}", tokenburn_core::appinfo::title()),
            <header class="top">brand(active: "settings".to_string())</header>
            <section>
                <h2>"Settings"</h2>
                <p class="dim">"Saved to " (tokenburn_core::appinfo::settings_file()) " and shared with the TUI and GUI."</p>
                <form class="filters" method="get" action="/">
                    <label>"currency "
                        <select name="currency">
                            for cur in tokenburn_core::CURRENCIES {
                                <option value=(cur.code) selected=(cur.code == currency.code)>(format!("{} · {}", cur.code, cur.name))</option>
                            }
                        </select>
                    </label>
                    <label>"theme "
                        <select name="theme">
                            for n in theme::names() {
                                <option value=(n) selected=(n == theme::name(theme_index))>(n)</option>
                            }
                        </select>
                    </label>
                    <button type="submit">"Save"</button>
                </form>
            </section>
            <section>
                <h2>"Spending limits"</h2>
                <p class="dim">"In US dollars; empty or 0 = off. Limits never block anything: you get progress bars, and a desktop notification when one is nearly reached or exceeded (checked every 30 s while this server runs)."</p>
                if !message.is_empty() {
                    <p class="limit-msg">(message.clone())</p>
                }
                <form class="filters limits-form" method="get" action="/settings">
                    <input type="hidden" name="save" value="1">
                    <label>"daily " <input type="text" inputmode="decimal" name="limit_daily" value=(daily.clone()) placeholder="off"></label>
                    <label>"weekly " <input type="text" inputmode="decimal" name="limit_weekly" value=(weekly.clone()) placeholder="off"></label>
                    <label>"monthly " <input type="text" inputmode="decimal" name="limit_monthly" value=(monthly.clone()) placeholder="off"></label>
                    <label><input type="checkbox" name="notify" value="on" checked=(notify)> " desktop notifications"</label>
                    <button type="submit">"Save limits"</button>
                </form>
            </section>
        )
    })
}

/// The live dashboard. Filter values are signals; because this shard reads
/// them on the server, changing a select re-renders only this shard. While a
/// connection is open the data region re-reads the logs every few seconds.
#[shard]
async fn live_dashboard(
    cx: &Cx,
    tool: String,
    bucket: String,
    window: String,
    theme: String,
    metric: String,
    currency: String,
    sel_bucket: String,
    sel_project: String,
) -> Result<impl View> {
    let tool = signal(cx, move || tool);
    let bucket = signal(cx, move || bucket);
    let window = signal(cx, move || window);
    let theme = signal(cx, move || theme);
    let metric = signal(cx, move || metric);
    let currency_sig = signal(cx, move || currency);
    // What the user clicked (empty = nothing). Untrusted like every signal: the
    // core only ever *compares* these against real labels, and drops stale ones.
    let sel_bucket = signal(cx, move || sel_bucket);
    let sel_project = signal(cx, move || sel_project);

    // Signal values come back from the browser: treat them as untrusted.
    // `parse` ignores anything that is not a known choice.
    let query = parse(Some(&tool.get()), Some(&bucket.get()), Some(&window.get()));
    let chart_metric = parse_metric(Some(&metric.get()));
    // Signal values are untrusted: only a known code counts, and a change is remembered.
    let currency = effective_currency(Some(&currency_sig.get()));
    let selection = parse_selection(Some(&sel_bucket.get()), Some(&sel_project.get()));
    // Same for the theme; an unknown name keeps the saved one. A change is remembered.
    let saved = theme::saved_index();
    let theme_index = theme::resolve(Some(&theme.get()), saved);
    if theme_index != saved {
        theme::save_index(theme_index);
    }

    Ok(view! {
        // Re-emitted on every theme change; later in the document than the <head> one.
        theme_style(index: theme_index)
        live_filters(tool: &tool, bucket: &bucket, window: &window, theme: &theme, metric: &metric, currency: &currency_sig)
        (live! {
            loop {
                let d = load(query, &selection, currency).await;
                let is_connected = connected(cx);
                let token = emit! { dashboard_body(d: &d, live: is_connected, metric: chart_metric, base: "", click_bucket: Some(&sel_bucket), click_project: Some(&sel_project)) }?;
                if !is_connected {
                    // HTTP render: emit once and finish.
                    break Ok(token);
                }
                tokio::time::sleep(REFRESH_EVERY).await;
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokenburn_core::{Bucket, Metric, Query, ToolFilter, Window};

    #[test]
    fn the_link_base_carries_the_view_but_not_the_theme_or_a_selection() {
        let q = Query {
            tool: ToolFilter::PI,
            bucket: Bucket::Day,
            window: Window::All,
        };
        assert_eq!(
            page_base(&q, Metric::Cost),
            "/?tool=pi&bucket=daily&window=all&metric=cost"
        );
        assert!(!page_base(&q, Metric::Tokens).contains("theme"));
    }
}
