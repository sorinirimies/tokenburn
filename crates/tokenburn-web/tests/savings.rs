//! The savings panel, card and JSON route, end to end. The sandbox holds an rtk
//! history of 40 commands × 50,000 tokens = 2M tokens, valued at the default
//! $3 per million input tokens = $6.00.

mod common;

use common::{get, start};

const ALL: &str = "tool=all&bucket=daily&window=all";

#[tokio::test]
async fn the_dashboard_shows_what_rtk_saved() {
    let addr = start().await;
    let html = get(addr, &format!("/?{ALL}&metric=saved")).await.body;
    assert!(html.contains("class=\"savings\""), "panel");
    assert!(html.contains("Saved"), "heading");
    assert!(html.contains("2,000,000"), "rtk tokens");
    assert!(html.contains("40 commands"), "events behind the number");
    assert!(html.contains("$6.00"), "2M tokens × $3/M");
    assert!(html.contains("estimate · valued at $3.00/M input tokens"));
    assert!(html.contains("class=\"card saved\""), "headline card");
    assert!(html.contains("of would-be cost"));
}

#[tokio::test]
async fn rtk_is_not_attributed_to_a_single_tool() {
    let addr = start().await;
    let html = get(addr, "/?tool=pi&bucket=daily&window=all&metric=saved")
        .await
        .body;
    assert!(
        !html.contains("class=\"savings\"") && !html.contains("class=\"label\">rtk<"),
        "no cache hits and no rtk for pi alone, so no panel and no rtk card"
    );
}

#[tokio::test]
async fn the_savings_json_route_reports_the_same_numbers() {
    let addr = start().await;
    let r = get(addr, &format!("/api/savings?{ALL}")).await;
    assert_eq!(r.status, 200);
    assert!(r.headers.contains("application/json"));
    let v: serde_json::Value = serde_json::from_str(&r.body).unwrap();
    assert_eq!(v["tokens"], 2_000_000);
    assert!((v["usd"].as_f64().unwrap() - 6.0).abs() < 1e-9);
    let src = &v["sources"][0];
    assert_eq!(src["source"], "rtk");
    assert_eq!(src["events"], 40);

    let none = get(addr, "/api/savings?tool=pi&window=all&metric=saved").await;
    let v: serde_json::Value = serde_json::from_str(&none.body).unwrap();
    assert_eq!(v["tokens"], 0);
    assert_eq!(v["sources"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn the_chart_dropdown_offers_saved_and_titles_the_chart() {
    let addr = start().await;
    let html = get(addr, &format!("/?{ALL}&metric=saved")).await.body;
    assert!(
        html.contains("<option value=\"saved\" selected"),
        "dropdown"
    );
    assert!(html.contains("Saved over time"), "chart title");
    let plain = get(addr, &format!("/?{ALL}")).await.body;
    assert!(plain.contains("<option value=\"saved\""), "always offered");
    assert!(plain.contains("Tokens over time"));
}

#[tokio::test]
async fn savings_are_listed_per_interval_and_the_interval_follows_the_filter() {
    let addr = start().await;
    // 40 rtk commands, one per minute from 10:00 to 10:39 on May 1st (UTC).
    let html = get(addr, "/?tool=all&bucket=minutely&window=all&metric=saved")
        .await
        .body;
    assert!(html.contains("Saved per minute"), "heading");
    assert!(html.contains("per-bucket"), "table");
    assert_eq!(
        html.matches("rtk $0.15").count(),
        24,
        "50,000 tokens × $3/M a minute; the newest 24 of 40 are listed"
    );

    let hourly = get(addr, "/?tool=all&bucket=hourly&window=all&metric=saved")
        .await
        .body;
    assert!(hourly.contains("Saved per hour"));
    assert!(
        hourly.matches("rtk $6.00").count() >= 1,
        "the same 40 commands in one hour"
    );

    let daily = get(addr, &format!("/?{ALL}&metric=saved")).await.body;
    assert!(daily.contains("Saved per day"));
    let monthly = get(addr, "/?tool=all&bucket=monthly&window=all&metric=saved")
        .await
        .body;
    assert!(monthly.contains("Saved per month"));
    assert!(monthly.contains("2026-05"));
}

#[tokio::test]
async fn the_minutely_bucket_is_offered_in_the_dropdown() {
    let addr = start().await;
    let html = get(addr, "/?tool=all&bucket=minutely&window=all&metric=saved")
        .await
        .body;
    assert!(
        html.contains("<option value=\"minutely\" selected"),
        "{html}"
    );
}

#[tokio::test]
async fn saved_details_show_only_for_the_saved_metric() {
    let addr = start().await;
    for metric in ["tokens", "cost"] {
        let html = get(addr, &format!("/?{ALL}&metric={metric}")).await.body;
        assert!(!html.contains("card saved"), "{metric}: no saved card");
        assert!(!html.contains("Saved per"), "{metric}: no saved panel");
    }
    let html = get(addr, &format!("/?{ALL}&metric=saved")).await.body;
    assert!(html.contains("card saved") && html.contains("Saved per day"));
    assert!(
        !html.contains("class=\"label\">total tokens<"),
        "the token cards give way"
    );
    let tokens = get(addr, &format!("/?{ALL}")).await.body;
    assert!(
        tokens.contains("class=\"label\">total tokens<")
            && !tokens.contains("class=\"label\">per turn")
    );
    let cost = get(addr, &format!("/?{ALL}&metric=cost")).await.body;
    assert!(
        cost.contains("class=\"label\">per turn") && cost.contains("class=\"label\">per 1M tokens")
    );
}

#[tokio::test]
async fn the_saved_panel_sits_below_the_charts_and_the_metric_is_with_the_filters() {
    let addr = start().await;
    let html = get(addr, &format!("/?{ALL}&metric=saved")).await.body;
    let charts = html.find("class=\"charts\"").expect("charts");
    let panel = html.find("class=\"savings\"").expect("panel");
    let report = html.find("Report ·").expect("report");
    assert!(
        charts < panel && panel < report,
        "charts, then saved, then the report"
    );
    let form = &html[html.find("class=\"filters\"").unwrap()..];
    let form = &form[..form.find("</form>").unwrap_or(form.len())];
    assert!(
        form.contains("name=\"metric\""),
        "same row as the other dropdowns"
    );
}
