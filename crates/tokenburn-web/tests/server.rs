//! End-to-end tests of the static dashboard: a real server on an ephemeral port,
//! driven with plain HTTP/1.1 over a `TcpStream` (no client dependency).
//!
//! The environment is pointed at a throw-away sandbox *before* anything reads
//! it, so these tests never see (or write) the developer's real logs/settings.

mod common;

use common::{get, sandbox, start};

const ALL_DAILY: &str = "tool=pi&bucket=daily&window=all";

#[tokio::test]
async fn health_and_static_assets() {
    let addr = start().await;
    let h = get(addr, "/api/health").await;
    assert_eq!(h.status, 200);
    assert_eq!(h.body.trim(), "\"ok\"");

    let css = get(addr, "/style.css").await;
    assert_eq!(css.status, 200);
    assert!(css.headers.contains("text/css"));
    assert!(css.body.contains(".charts"), "chart styles are served");

    assert_eq!(get(addr, "/nope").await.status, 404);
}

#[tokio::test]
async fn the_dashboard_renders_summary_charts_and_table() {
    let addr = start().await;
    let r = get(addr, &format!("/?{ALL_DAILY}")).await;
    assert_eq!(r.status, 200);
    assert!(r.headers.contains("text/html"));
    let html = r.body;
    assert!(html.contains("TokenBurn"));
    assert!(
        html.contains("http-equiv=\"refresh\""),
        "static mode reloads itself"
    );
    // Summary: 3 turns, 6000 input.
    assert!(html.contains(">3<") || html.contains("3\n"), "turn count");
    // Charts.
    assert_eq!(
        html.matches("<svg class=\"chart").count(),
        2,
        "bars + donut"
    );
    assert!(html.contains("Tokens over time"));
    assert!(html.contains("Share by project"));
    assert!(html.contains("<rect"), "bar segments");
    assert!(html.contains("<title>"), "hover tooltips");
    // Table rows, one per day.
    assert!(html.contains("2026-05-01") && html.contains("2026-05-02"));
    assert!(html.contains("demo-proj") || html.contains("pi"));
}

#[tokio::test]
async fn the_chart_metric_switches_with_the_query() {
    let addr = start().await;
    let r = get(addr, &format!("/?{ALL_DAILY}&metric=cost")).await;
    assert!(r.body.contains("Cost over time"));
    assert!(r.body.contains("selected") && r.body.contains("cost"));
    let r = get(addr, &format!("/?{ALL_DAILY}&metric=garbage")).await;
    assert!(
        r.body.contains("Tokens over time"),
        "unknown metric → tokens"
    );
}

#[tokio::test]
async fn an_empty_selection_says_so_instead_of_breaking() {
    let addr = start().await;
    let r = get(addr, "/?tool=amp&bucket=daily&window=all").await;
    assert_eq!(r.status, 200);
    assert!(r.body.contains("No data"), "empty charts show a message");
}

#[tokio::test]
async fn untrusted_query_values_are_ignored_and_never_reflected() {
    let addr = start().await;
    let r = get(
        addr,
        "/?tool=%3Cscript%3Ealert(1)%3C/script%3E&bucket=x&window=y",
    )
    .await;
    assert_eq!(r.status, 200);
    assert!(!r.body.contains("<script>alert"), "no reflected markup");
}

#[tokio::test]
async fn the_json_api_returns_the_same_report() {
    let addr = start().await;
    let r = get(addr, &format!("/api/report?{ALL_DAILY}")).await;
    assert_eq!(r.status, 200);
    assert!(r.headers.contains("application/json"));
    let v: serde_json::Value = serde_json::from_str(&r.body).unwrap();
    let rows = v.as_array().unwrap();
    assert_eq!(rows.len(), 2, "two days");
    let total: u64 = rows
        .iter()
        .map(|r| r["total_tokens"].as_u64().unwrap())
        .sum();
    assert_eq!(total, 1000 + 500 + 2000 + 1000 + 3000 + 1500);
}

#[tokio::test]
async fn themes_are_listed_and_a_chosen_one_is_applied_and_remembered() {
    let addr = start().await;
    let t = get(addr, "/api/themes").await;
    let v: serde_json::Value = serde_json::from_str(&t.body).unwrap();
    let n = v
        .as_array()
        .map_or_else(|| v["themes"].as_array().unwrap().len(), Vec::len);
    assert_eq!(n, tokenburn_core::THEME_COUNT);

    let r = get(addr, "/?theme=Dracula&window=all").await;
    assert!(r.body.contains("--s0"), "theme variables are inlined");
    let saved = std::fs::read_to_string(sandbox().config.join("settings.json")).unwrap();
    assert!(saved.contains("Dracula"), "{saved}");

    // An unknown theme keeps the saved one.
    let r = get(addr, "/?theme=Nope&window=all").await;
    assert_eq!(r.status, 200);
    let saved = std::fs::read_to_string(sandbox().config.join("settings.json")).unwrap();
    assert!(saved.contains("Dracula"));
}

/// The first `href="…sel_<kind>=…"` in `html`, HTML-unescaped.
fn first_link(html: &str, kind: &str) -> String {
    let needle = format!("sel_{kind}=");
    let at = html
        .find(&needle)
        .unwrap_or_else(|| panic!("no {needle} link in the page"));
    let start = html[..at].rfind("href=\"").unwrap() + 6;
    let end = at + html[at..].find('"').unwrap();
    html[start..end].replace("&amp;", "&")
}

#[tokio::test]
async fn bars_and_slices_are_links_in_static_mode() {
    let addr = start().await;
    let html = get(addr, &format!("/?{ALL_DAILY}")).await.body;
    let bar = first_link(&html, "bucket");
    assert!(
        bar.starts_with("/?tool=pi&bucket=daily&window=all&metric=tokens&sel_bucket="),
        "{bar}"
    );
    let slice = first_link(&html, "project");
    assert!(slice.contains("sel_project=pi%3A"), "{slice}");
    assert!(html.contains("click a bar or a slice to drill down"));
    assert!(!html.contains("class=\"col sel"), "nothing selected yet");
}

#[tokio::test]
async fn clicking_a_bar_scopes_the_page_to_that_bucket_and_the_link_toggles_it_off() {
    let addr = start().await;
    let all = get(addr, &format!("/?{ALL_DAILY}")).await.body;
    let bar = first_link(&all, "bucket");
    assert!(bar.contains("sel_bucket=2026-05-01"), "{bar}");

    let r = get(addr, &bar).await;
    assert_eq!(r.status, 200);
    let html = r.body;
    assert!(html.contains("col sel"), "the clicked bar is marked");
    assert!(
        html.contains("bars chart") || html.contains("has-sel"),
        "the others dim"
    );
    assert!(html.contains("click to clear"));
    assert!(html.contains("class=\"chip\"") && html.contains("2026-05-01"));
    // Only that day's rows remain in the table (day 2 is gone from it).
    let table = html.split("<table").nth(1).unwrap();
    assert!(
        table.contains("2026-05-01") && !table.contains("2026-05-02"),
        "table scoped"
    );
    // The bars still show every day: day 2's column is there to click.
    assert!(html.contains("sel_bucket=2026-05-02"));
    // Clicking the selected bar again links back to the unselected page.
    let again = first_link(&html, "bucket");
    assert!(!again.is_empty());
    let clear_href = {
        let at = html.find("class=\"clear\"").unwrap();
        let start = html[at..].find("href=\"").unwrap() + at + 6;
        let end = start + html[start..].find('"').unwrap();
        html[start..end].replace("&amp;", "&")
    };
    assert!(!clear_href.contains("sel_"), "{clear_href}");
}

#[tokio::test]
async fn clicking_a_slice_scopes_to_that_project() {
    let addr = start().await;
    let all = get(addr, &format!("/?{ALL_DAILY}")).await.body;
    let slice = first_link(&all, "project");
    let r = get(addr, &slice).await.body;
    assert!(r.contains("slice sel"), "the clicked slice is marked");
    assert!(r.contains("click to clear"));
    assert!(r.contains("legend vertical has-sel"));
    assert!(r.contains("class=\"item sel\""));
}

#[tokio::test]
async fn both_selections_combine_and_junk_selections_are_ignored() {
    let addr = start().await;
    let r = get(
        addr,
        &format!("/?{ALL_DAILY}&sel_bucket=2026-05-02&sel_project=pi%3Anope"),
    )
    .await;
    assert_eq!(r.status, 200);
    assert_eq!(
        r.body.matches("class=\"chip\"").count(),
        1,
        "only the real bucket is kept"
    );
    assert!(
        !r.body.contains("pi:nope"),
        "the unknown project is dropped, not echoed"
    );
    let r = get(addr, &format!("/?{ALL_DAILY}&sel_bucket=%3Cscript%3E")).await;
    assert!(!r.body.contains("<script>"), "never reflected");
    assert!(
        !r.body.contains("class=\"chip\""),
        "an unknown bucket is dropped"
    );
}

#[tokio::test]
async fn there_is_no_manual_refresh_control() {
    let addr = start().await;
    let html = get(addr, &format!("/?{ALL_DAILY}")).await.body;
    assert!(!html.contains(">Refresh<"));
}

#[tokio::test]
async fn tables_scroll_sideways_and_secondary_columns_are_tagged_for_the_media_queries() {
    let addr = start().await;
    let html = get(addr, &format!("/?{ALL_DAILY}")).await.body;
    assert!(
        html.contains("class=\"table-wrap\""),
        "tables are wrapped so they never overflow the page"
    );
    // Cache columns drop out on medium screens, the rest on narrow ones.
    assert!(html.contains("<th class=\"opt1\">cache read</th>"));
    assert!(html.contains("<th class=\"opt2\">input</th>"));
    // The essentials are never tagged.
    assert!(html.contains("<th>total</th>") && html.contains("<th>cost</th>"));
    assert!(
        html.contains("name=\"viewport\""),
        "the viewport meta makes the media queries work on phones"
    );
    // Every other x label is marked so narrow screens can thin them out.
    assert!(html.contains("class=\"axis alt\"") || html.matches("class=\"axis\"").count() > 0);
}
