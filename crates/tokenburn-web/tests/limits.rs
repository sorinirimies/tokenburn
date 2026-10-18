//! Spending limits end to end: the Settings form, the progress bars, the JSON route
//! and the watcher. One test owns the saved settings, so the steps cannot interleave.
//! The sandbox gets one $0.50 turn happening right now.

mod common;

use common::{get, sandbox, start};

const DASH: &str = "tool=all&bucket=daily&window=all";

#[tokio::test]
async fn limits_are_set_shown_and_watched() {
    let root = sandbox().config.parent().unwrap().to_path_buf();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    std::fs::write(
        root.join("sessions/--demo-proj--/now.jsonl"),
        format!(
            r#"{{"timestamp":"{now}","message":{{"usage":{{"input":10,"output":5,"cacheRead":0,"cacheWrite":0,"cost":{{"total":0.5}}}}}}}}"#
        ),
    )
    .unwrap();
    let addr = start().await;

    // No limits: no panel, and the settings form is empty.
    let html = get(addr, &format!("/?{DASH}")).await.body;
    assert!(!html.contains("class=\"limits\""), "{html}");
    let page = get(addr, "/settings").await.body;
    assert!(page.contains("Spending limits") && page.contains("name=\"limit_daily\""));
    assert!(!page.contains("Limits saved"));

    // Save: daily $0.40 (already over), weekly $100, monthly off. Notifications off, so
    // the watcher below sends no real desktop notification.
    let saved = get(
        addr,
        "/settings?save=1&limit_daily=0.4&limit_weekly=100&limit_monthly=",
    )
    .await
    .body;
    assert!(saved.contains("Limits saved."), "{saved}");
    let file = std::fs::read_to_string(root.join("config/settings.json")).unwrap();
    assert!(file.contains("0.4") && file.contains("100"), "{file}");
    assert!(
        file.contains("\"notify\": false"),
        "unchecked box = off: {file}"
    );
    assert!(
        saved.contains("value=\"0.4\"") && saved.contains("value=\"100\""),
        "fields keep"
    );

    // A bad number is reported and nothing changes.
    let bad = get(addr, "/settings?save=1&limit_daily=abc&limit_weekly=5")
        .await
        .body;
    assert!(bad.contains("daily limit must be a number"), "{bad}");
    let file = std::fs::read_to_string(root.join("config/settings.json")).unwrap();
    assert!(
        file.contains("0.4") && file.contains("100"),
        "unchanged: {file}"
    );

    // The dashboard shows a bar per limit, coloured by state.
    let html = get(addr, &format!("/?{DASH}")).await.body;
    assert!(html.contains("class=\"limits\""), "{html}");
    assert!(
        html.contains("limit over") && html.contains("exceeded"),
        "{html}"
    );
    assert!(html.contains("limit ok"), "weekly is fine: {html}");
    assert!(html.contains("daily · $0.50 of $0.40 (125%)"), "{html}");
    assert!(
        html.contains("<progress max=\"100\" value=\"100\""),
        "bar is capped: {html}"
    );
    assert!(!html.contains("monthly ·"), "no monthly limit");
    let limits = html.find("class=\"limits\"").unwrap();
    let cards = html.find("class=\"cards\"").unwrap();
    assert!(limits < cards, "limits above the figures");

    // JSON, US dollars.
    let json: serde_json::Value =
        serde_json::from_str(&get(addr, "/api/limits").await.body).unwrap();
    let st = json["statuses"].as_array().unwrap();
    assert_eq!(st.len(), 2);
    assert_eq!(st[0]["period"], "day");
    assert_eq!(st[0]["level"], "over");
    assert_eq!(st[1]["level"], "ok");
    assert_eq!(json["limits"]["daily"], 0.4);

    // The watcher raises the day alert once, whoever asks next hears nothing.
    let alerts = tokenburn_web::data::watch_limits().await;
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].period, tokenburn_core::Period::Day);
    assert!(tokenburn_web::data::watch_limits().await.is_empty());
    // The page still shows the state: it is not consumed by the alarm.
    assert!(
        get(addr, &format!("/?{DASH}"))
            .await
            .body
            .contains("limit over")
    );
}
