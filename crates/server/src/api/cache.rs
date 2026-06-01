//! API endpoints for cache management.
//!
//! All endpoints are protected by authentication middleware at the API layer.

use crate::api::response::{err, ok};
use crate::AppState;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Router;

/// Get cache statistics.
#[tracing::instrument(skip(state))]
pub async fn stats(State(state): State<AppState>) -> impl IntoResponse {
    match state.cache.stats().await {
        Ok(stats) => ok(stats).into_response(),
        Err(e) => err(format!("Failed to get cache stats: {}", e)).into_response(),
    }
}

/// Clear all cache.
#[tracing::instrument(skip(state))]
pub async fn clear(State(state): State<AppState>) -> impl IntoResponse {
    match state.cache.clear_all().await {
        Ok(()) => ok(serde_json::json!({"message": "Cache cleared"})).into_response(),
        Err(e) => err(format!("Failed to clear cache: {}", e)).into_response(),
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/console/api/cache/stats", axum::routing::get(stats))
        .route("/console/api/cache/clear", axum::routing::post(clear))
}
