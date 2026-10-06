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
    components::{dashboard_body, filters, live_filters, theme_style},
    data::{REFRESH_EVERY, load},
    params::{UsageParams, parse, parse_metric, parse_selection},
    theme,
};

use tokenburn_core::settings;

/// HTML shell shared by every page.
#[component]
pub async fn document(interactive: bool, theme: usize, child: Child<'_>) -> Result<impl View> {
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
                <title>"TokenBurn — AI token usage"</title>
                <link rel="stylesheet" href="/style.css">
                // The theme's CSS variables override the stylesheet's defaults.
                theme_style(index: theme)
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

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    let params = query_params::<UsageParams>(cx)?;
    let interactive = is_interactive(cx);
    let metric = parse_metric(params.metric.as_deref());

    // An explicit, valid `?theme=` wins and is remembered (shared with the TUI and GUI).
    let saved = settings::theme_index();
    let theme_index = theme::resolve(params.theme.as_deref(), saved);
    if theme_index != saved {
        let _ = settings::save_theme(theme_index);
    }

    // Static fallback loads once here; the live path loads inside the shard.
    let selection = params.selection();
    let snapshot = if interactive {
        None
    } else {
        Some(load(params.to_query(), &selection).await)
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
        document(interactive: interactive, theme: theme_index,
            if let Some(d) = &snapshot {
                <header class="top">
                    <h1>"🔥 TokenBurn"</h1>
                    filters(query: d.query, theme: theme_index, metric: metric)
                </header>
                dashboard_body(d: d, live: false, metric: metric, base: &base, click_bucket: None, click_project: None)
            } else {
                <header class="top"><h1>"🔥 TokenBurn"</h1></header>
                live_dashboard(tool: tool, bucket: bucket, window: window, theme: theme::names()[theme_index].to_string(), metric: metric.label().to_string(), sel_bucket: selection.bucket.clone().unwrap_or_default(), sel_project: selection.project.clone().unwrap_or_default())
            }
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
    sel_bucket: String,
    sel_project: String,
) -> Result<impl View> {
    let tool = signal(cx, move || tool);
    let bucket = signal(cx, move || bucket);
    let window = signal(cx, move || window);
    let theme = signal(cx, move || theme);
    let metric = signal(cx, move || metric);
    // What the user clicked (empty = nothing). Untrusted like every signal: the
    // core only ever *compares* these against real labels, and drops stale ones.
    let sel_bucket = signal(cx, move || sel_bucket);
    let sel_project = signal(cx, move || sel_project);

    // Signal values come back from the browser: treat them as untrusted.
    // `parse` ignores anything that is not a known choice.
    let query = parse(Some(&tool.get()), Some(&bucket.get()), Some(&window.get()));
    let chart_metric = parse_metric(Some(&metric.get()));
    let selection = parse_selection(Some(&sel_bucket.get()), Some(&sel_project.get()));
    // Same for the theme; an unknown name keeps the saved one. A change is remembered.
    let saved = settings::theme_index();
    let theme_index = theme::resolve(Some(&theme.get()), saved);
    if theme_index != saved {
        let _ = settings::save_theme(theme_index);
    }

    Ok(view! {
        // Re-emitted on every theme change; later in the document than the <head> one.
        theme_style(index: theme_index)
        live_filters(tool: &tool, bucket: &bucket, window: &window, theme: &theme, metric: &metric)
        (live! {
            loop {
                let d = load(query, &selection).await;
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
