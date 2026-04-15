use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use burncloud_service_router_log::{BillingService, RouterLogService};
use serde::{Deserialize, Serialize};

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

#[derive(Serialize)]
struct LogPage {
    data: Vec<burncloud_service_router_log::RouterLog>,
    page: i32,
    page_size: i32,
}

#[derive(Serialize)]
struct UserUsage {
    user_id: String,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
}

#[derive(Serialize)]
struct ApiError {
    error: String,
}

#[derive(Serialize)]
struct PriceSyncResult {
    models_synced: usize,
    currencies_synced: usize,
    tiers_synced: usize,
    errors: usize,
    source: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/console/api/logs", get(list_logs))
        .route("/console/api/usage/{user_id}", get(get_user_usage))
        .route(
            "/console/internal/billing/summary",
            get(billing_summary_handler),
        )
        .route("/console/internal/prices/sync", post(price_sync_handler))
}

async fn list_logs(
    State(state): State<AppState>,
    Query(params): Query<Pagination>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(50);
    let offset = (page - 1) * page_size;

    match RouterLogService::get(&state.db, page_size, offset).await {
        Ok(data) => Json(LogPage {
            data,
            page,
            page_size,
        })
        .into_response(),
        Err(e) => Json(ApiError {
            error: e.to_string(),
        })
        .into_response(),
    }
}

async fn get_user_usage(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    match RouterLogService::get_usage_by_user(&state.db, &user_id).await {
        Ok((prompt_tokens, completion_tokens)) => Json(UserUsage {
            total_tokens: prompt_tokens + completion_tokens,
            user_id,
            prompt_tokens,
            completion_tokens,
        })
        .into_response(),
        Err(e) => Json(ApiError {
            error: e.to_string(),
        })
        .into_response(),
    }
}

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

async fn price_sync_handler(State(state): State<AppState>) -> Response {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    if state.force_sync_tx.send(reply_tx).await.is_err() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError {
                error: "Price sync task is not running".to_string(),
            }),
        )
            .into_response();
    }
    match tokio::time::timeout(std::time::Duration::from_secs(60), reply_rx).await {
        Ok(Ok(result)) => Json(PriceSyncResult {
            models_synced: result.models_synced,
            currencies_synced: result.currencies_synced,
            tiers_synced: result.tiered_pricing_synced,
            errors: result.errors,
            source: result.source,
        })
        .into_response(),
        Ok(Err(_)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: "Sync task dropped the reply channel".to_string(),
            }),
        )
            .into_response(),
        Err(_) => (
            StatusCode::GATEWAY_TIMEOUT,
            Json(ApiError {
                error: "Price sync timed out after 60s".to_string(),
            }),
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

    match BillingService::get_billing_summary(&state.db, start, end).await {
        Ok(summary) => Json(summary).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}
