use crate::api::auth::Claims;
use crate::api::response::{err, ok};
use crate::AppState;
use axum::{
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    middleware,
    response::{IntoResponse, Json, Response},
    routing::get,
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

pub fn routes() -> Router<AppState> {
    let public_billing = Router::new()
        .route("/api/billing/summary", get(public_billing_summary_handler))
        .layer(middleware::from_fn(crate::auth_middleware));

    Router::new()
        .route("/console/api/logs", get(list_logs))
        .route("/console/api/usage/{user_id}", get(get_user_usage))
        .route(
            "/console/internal/billing/summary",
            get(billing_summary_handler),
        )
        .merge(public_billing)
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

async fn public_billing_summary_handler(
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
