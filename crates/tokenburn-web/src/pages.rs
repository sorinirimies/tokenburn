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
    params::{UsageParams, parse},
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
                    // Static fallback: no runtime, so just reload periodically.
                    <meta http-equiv="refresh" content="60">
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

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    let params = query_params::<UsageParams>(cx)?;
    let interactive = is_interactive(cx);

    // An explicit, valid `?theme=` wins and is remembered (shared with the TUI and GUI).
    let saved = settings::theme_index();
    let theme_index = theme::resolve(params.theme.as_deref(), saved);
    if theme_index != saved {
        let _ = settings::save_theme(theme_index);
    }

    // Static fallback loads once here; the live path loads inside the shard.
    let snapshot = if interactive {
        None
    } else {
        Some(load(params.to_query()).await)
    };

    // Initial control values come from the URL; signals take over afterwards.
    let tool = params.tool.clone().unwrap_or_else(|| "all".into());
    let bucket = params.bucket.clone().unwrap_or_else(|| "total".into());
    let window = params.window.clone().unwrap_or_else(|| "auto".into());

    Ok(view! {
        document(interactive: interactive, theme: theme_index,
            if let Some(d) = &snapshot {
                <header class="top">
                    <h1>"🔥 TokenBurn"</h1>
                    filters(query: d.query, theme: theme_index)
                </header>
                dashboard_body(d: d, live: false)
            } else {
                <header class="top"><h1>"🔥 TokenBurn"</h1></header>
                live_dashboard(tool: tool, bucket: bucket, window: window, theme: theme::names()[theme_index].to_string())
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
) -> Result<impl View> {
    let tool = signal(cx, move || tool);
    let bucket = signal(cx, move || bucket);
    let window = signal(cx, move || window);
    let theme = signal(cx, move || theme);

    // Signal values come back from the browser: treat them as untrusted.
    // `parse` ignores anything that is not a known choice.
    let query = parse(Some(&tool.get()), Some(&bucket.get()), Some(&window.get()));
    // Same for the theme; an unknown name keeps the saved one. A change is remembered.
    let saved = settings::theme_index();
    let theme_index = theme::resolve(Some(&theme.get()), saved);
    if theme_index != saved {
        let _ = settings::save_theme(theme_index);
    }

    Ok(view! {
        // Re-emitted on every theme change; later in the document than the <head> one.
        theme_style(index: theme_index)
        live_filters(tool: &tool, bucket: &bucket, window: &window, theme: &theme)
        (live! {
            loop {
                let d = load(query).await;
                let is_connected = connected(cx);
                let token = emit! { dashboard_body(d: &d, live: is_connected) }?;
                if !is_connected {
                    // HTTP render: emit once and finish.
                    break Ok(token);
                }
                tokio::time::sleep(REFRESH_EVERY).await;
            }
        })
    })
}
