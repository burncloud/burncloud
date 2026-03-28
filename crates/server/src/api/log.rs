use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use burncloud_database_router::RouterDatabase;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct Pagination {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
}

#[derive(Deserialize)]
struct BillingSummaryParams {
    start: Option<String>,
    end: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/console/api/logs", get(list_logs))
        .route("/console/api/usage/{user_id}", get(get_user_usage))
        .route(
            "/console/internal/billing/summary",
            get(billing_summary_handler),
        )
        .route(
            "/console/internal/prices/sync",
            post(price_sync_handler),
        )
}

async fn list_logs(State(state): State<AppState>, Query(params): Query<Pagination>) -> Json<Value> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(50);
    let offset = (page - 1) * page_size;

    match RouterDatabase::get_logs(&state.db, page_size, offset).await {
        Ok(logs) => Json(json!({
            "data": logs,
            "page": page,
            "page_size": page_size
        })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn get_user_usage(State(state): State<AppState>, Path(user_id): Path<String>) -> Json<Value> {
    match RouterDatabase::get_usage_by_user(&state.db, &user_id).await {
        Ok((prompt, completion)) => Json(json!({
            "user_id": user_id,
            "prompt_tokens": prompt,
            "completion_tokens": completion,
            "total_tokens": prompt + completion
        })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

/// GET /console/internal/billing/summary
///
/// Returns per-model billing summary for reconciliation with upstream providers.
/// Requires `x-internal-secret` header when `BURNCLOUD_INTERNAL_SECRET` env var is set.
async fn billing_summary_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<BillingSummaryParams>,
) -> Response {
    billing_summary_inner(
        &state,
        &headers,
        params.start.as_deref(),
        params.end.as_deref(),
        std::env::var("BURNCLOUD_INTERNAL_SECRET").ok().as_deref(),
    )
    .await
}

/// POST /console/internal/prices/sync
///
/// Triggers an immediate forced price sync. Waits up to 60 seconds for completion.
async fn price_sync_handler(State(state): State<AppState>) -> Response {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    if state.force_sync_tx.send(reply_tx).await.is_err() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Price sync task is not running" })),
        )
            .into_response();
    }
    match tokio::time::timeout(std::time::Duration::from_secs(60), reply_rx).await {
        Ok(Ok(result)) => Json(json!({
            "models_synced": result.models_synced,
            "currencies_synced": result.currencies_synced,
            "tiers_synced": result.tiered_pricing_synced,
            "errors": result.errors,
            "source": result.source,
        }))
        .into_response(),
        Ok(Err(_)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Sync task dropped the reply channel" })),
        )
            .into_response(),
        Err(_) => (
            StatusCode::GATEWAY_TIMEOUT,
            Json(json!({ "error": "Price sync timed out after 60s" })),
        )
            .into_response(),
    }
}

async fn billing_summary_inner(
    state: &AppState,
    headers: &HeaderMap,
    start: Option<&str>,
    end: Option<&str>,
    secret: Option<&str>,
) -> Response {
    if let Some(expected) = secret {
        let provided = headers
            .get("x-internal-secret")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if provided != expected {
            return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
        }
    }

    match burncloud_database_router::get_billing_summary(&state.db, start, end).await {
        Ok(summary) => Json(json!({
            "period": {
                "start": summary.period_start,
                "end": summary.period_end,
            },
            "pre_migration_requests": summary.pre_migration_requests,
            "models": summary.models.iter().map(|m| json!({
                "model": m.model,
                "requests": m.requests,
                "prompt_tokens": m.prompt_tokens,
                "cache_read_tokens": m.cache_read_tokens,
                "completion_tokens": m.completion_tokens,
                "reasoning_tokens": m.reasoning_tokens,
                "cost_usd": m.cost_usd,
            })).collect::<Vec<_>>(),
            "total_cost_usd": summary.total_cost_usd,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
