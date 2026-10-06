//! The currency selector, end to end. A separate test binary (so a separate sandbox
//! and settings file): choosing a currency is *saved*, which would race the other
//! tests if they shared one.

mod common;

use common::{get, sandbox, start};

const ALL_DAILY: &str = "tool=pi&bucket=daily&window=all";

/// One test owns the saved setting, so the steps cannot interleave.
#[tokio::test]
async fn costs_follow_the_selected_currency_and_it_is_remembered() {
    let addr = start().await;
    let settings = sandbox().config.join("settings.json");

    // Default: dollars, no "approximate" marker, and the select offers every currency.
    let usd = get(addr, &format!("/?{ALL_DAILY}")).await.body;
    assert!(
        usd.contains("$0.75") || usd.contains("$0.5"),
        "dollar costs: {usd}"
    );
    assert!(!usd.contains("cost ≈"));
    assert!(usd.contains("name=\"currency\""), "the selector is there");
    for code in ["USD", "EUR", "GBP", "JPY", "CHF"] {
        assert!(
            usd.contains(&format!("value=\"{code}\"")),
            "{code} is offered"
        );
    }

    // Choose euros: converted amounts, a € symbol, and the cost label says "≈".
    let eur = get(addr, &format!("/?{ALL_DAILY}&currency=EUR")).await.body;
    assert!(eur.contains(">€1.38<"), "1.50 USD at 0.92: {eur}");
    assert!(eur.contains("cost ≈"), "built-in rates are estimates");
    assert!(
        !eur.contains(">$0."),
        "no dollar amounts left in the cards and tables"
    );
    // The chart axis and tooltips use it too.
    let cost_chart = get(addr, &format!("/?{ALL_DAILY}&currency=EUR&metric=cost"))
        .await
        .body;
    assert!(cost_chart.contains('€') && cost_chart.contains("Cost over time"));
    assert!(!cost_chart.contains(">$"), "axis labels are in euros");

    // The choice is saved, shared with the other front-ends, and used without a param.
    let saved = std::fs::read_to_string(&settings).unwrap();
    assert!(saved.contains("\"currency\": \"EUR\""), "{saved}");
    let again = get(addr, &format!("/?{ALL_DAILY}")).await.body;
    assert!(
        again.contains('€'),
        "the saved currency applies on the next visit"
    );
    assert!(
        again.contains("<option value=\"EUR\" selected=\"\">"),
        "the selector shows the current one"
    );

    // Whole-number currencies have no decimals.
    let jpy = get(addr, &format!("/?{ALL_DAILY}&currency=jpy")).await.body;
    assert!(jpy.contains('¥') && !jpy.contains("¥0."), "{jpy}");

    // Junk keeps the saved one, never an error page.
    let junk = get(addr, &format!("/?{ALL_DAILY}&currency=%3Cscript%3E")).await;
    assert_eq!(junk.status, 200);
    assert!(!junk.body.contains("<script>") && junk.body.contains('¥'));

    // The user's own rate (settings.json "rates") wins and drops the "≈".
    std::fs::write(&settings, r#"{"currency":"EUR","rates":{"EUR":2.0}}"#).unwrap();
    let own = get(addr, &format!("/?{ALL_DAILY}")).await.body;
    assert!(own.contains(">€3.00<"), "1.50 USD at 2.0: {own}");
    assert!(!own.contains("cost ≈"), "your own rate is not an estimate");

    // And back to dollars.
    let back = get(addr, &format!("/?{ALL_DAILY}&currency=USD")).await.body;
    assert!(back.contains(">$1.50<") && !back.contains('€'));
    // The JSON API stays in dollars whatever is selected.
    let api = get(addr, &format!("/api/report?{ALL_DAILY}&currency=EUR"))
        .await
        .body;
    assert!(api.contains("cost_usd"));
}
