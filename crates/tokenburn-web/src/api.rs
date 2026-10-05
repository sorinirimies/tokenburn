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
    Ok(Json(load(query).await.report))
}

/// Liveness probe.
#[route(GET "/api/health")]
async fn health() -> Result<Json<&'static str>> {
    Ok(Json("ok"))
}

#[route(GET "/style.css")]
async fn style() -> Result<Css<&'static str>> {
    Ok(Css(CSS))
}
