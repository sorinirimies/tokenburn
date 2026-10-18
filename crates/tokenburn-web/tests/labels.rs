//! The drop-downs say what they choose: every filter has a plain-language label, on
//! the server-rendered page and on the live one.

mod common;

use common::{get, start};

const LABELS: [&str; 6] = [
    "chart shows ",
    "tool ",
    "group by ",
    "period ",
    "currency ",
    "theme ",
];

#[tokio::test]
async fn every_drop_down_on_the_dashboard_is_labelled() {
    let addr = start().await;
    let html = get(addr, "/?tool=all&bucket=daily&window=all").await.body;
    for l in LABELS {
        assert!(
            html.contains(&format!("<label>{l}")),
            "missing '{l}': {html}"
        );
    }
    // The old, vaguer words are gone.
    for old in ["<label>chart <", "<label>view ", "<label>window "] {
        assert!(!html.contains(old), "stale label {old}");
    }
    // One label per select, in toolbar order.
    let order: Vec<usize> = LABELS
        .iter()
        .map(|l| html.find(&format!("<label>{l}")).unwrap())
        .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
    assert!(html.matches("<select").count() >= 6);
}

#[tokio::test]
async fn the_settings_page_labels_its_drop_downs_too() {
    let addr = start().await;
    let html = get(addr, "/settings").await.body;
    for l in ["<label>currency ", "<label>theme "] {
        assert!(html.contains(l), "{l}: {html}");
    }
}
