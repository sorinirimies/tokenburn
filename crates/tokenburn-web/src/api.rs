//! JSON and static routes.

use topcoat::{
    Result,
    context::Cx,
    router::{
        content::{Css, Json},
        query_params, route,
    },
};

use crate::{data::load, params::UsageParams, style::CSS};

/// The same report as JSON: `/api/report?tool=pi&bucket=daily`.
#[route(GET "/api/report")]
async fn report(cx: &Cx) -> Result<Json<Vec<tokenburn_core::ReportRow>>> {
    let query = query_params::<UsageParams>(cx)?.to_query();
    Ok(Json(
        load(query, &Default::default(), tokenburn_core::Currency::USD)
            .await
            .report,
    ))
}

/// Liveness probe.
#[route(GET "/api/health")]
async fn health() -> Result<Json<&'static str>> {
    Ok(Json("ok"))
}

/// Every colour theme with its palette (the same ones the TUI and GUI offer).
#[route(GET "/api/themes")]
async fn themes() -> Result<Json<serde_json::Value>> {
    Ok(Json(crate::theme::catalogue()))
}

#[route(GET "/style.css")]
async fn style() -> Result<Css<&'static str>> {
    Ok(Css(CSS))
}

/// Extra styling for the Cyberpunk theme, imported by the inline theme `<style>`.
#[route(GET "/cyberpunk.css")]
async fn cyberpunk_style() -> Result<Css<&'static str>> {
    Ok(Css(crate::theme::CYBERPUNK_CSS))
}

/// What prompt caching, rtk and pi compaction saved, valued at the saved input
/// price, in US dollars: `/api/savings?tool=all&window=month`.
#[route(GET "/api/savings")]
async fn savings(cx: &Cx) -> Result<Json<tokenburn_core::Savings>> {
    let query = query_params::<UsageParams>(cx)?.to_query();
    Ok(Json(
        load(query, &Default::default(), tokenburn_core::Currency::USD)
            .await
            .savings,
    ))
}

/// Spending limits and how much of each is used (US dollars): `/api/limits`.
#[route(GET "/api/limits")]
async fn limits() -> Result<Json<tokenburn_core::LimitCheck>> {
    Ok(Json(
        crate::data::check_limits(tokenburn_core::Currency::USD).await,
    ))
}
