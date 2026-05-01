//! Public billing API — `GET /api/billing/summary`
//!
//! Extracted from `log.rs` so billing routes live in their own module.
//! The internal console endpoint (`/console/internal/billing/summary`) stays
//! in `log.rs` alongside the other console routes.

use crate::api::auth::Claims;
use crate::api::response::{err, ok};
use crate::AppState;
use axum::{
    extract::{Extension, Query, State},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use burncloud_service_router_log::BillingService;
use serde::Deserialize;

#[derive(Deserialize)]
struct BillingSummaryParams {
    start: Option<String>,
    end: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/billing/summary", get(billing_summary_handler))
        .layer(middleware::from_fn(crate::auth_middleware))
}

async fn billing_summary_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<BillingSummaryParams>,
) -> Response {
    match BillingService::get_billing_summary_for_user(
        &state.db,
        &claims.sub,
        params.start.as_deref(),
        params.end.as_deref(),
    )
    .await
    {
        Ok(summary) => ok(summary).into_response(),
        Err(e) => err(e.to_string()).into_response(),
    }
}
