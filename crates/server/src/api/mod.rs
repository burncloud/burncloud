use crate::AppState;
pub mod security;

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};

pub mod auth;
pub mod billing;
pub mod channel;
pub mod log;
pub mod monitor;
pub mod openapi;
pub mod response;
pub mod token;
pub mod user;

/// Fallback handler for unmatched /console/api/* requests
/// Returns 404 instead of being caught by LiveView's catch-all
async fn api_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "API endpoint not found")
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .merge(auth::routes())
        .merge(billing::routes())
        .merge(channel::routes())
        .merge(token::routes())
        .merge(log::routes())
        .merge(monitor::routes())
        .merge(user::routes())
        .merge(security::security_routes())
        .merge(openapi::routes())
        // Catch-all for any unmatched /console/api/* paths
        // This prevents LiveView from returning HTML for non-existent API endpoints
        .route("/console/api/{*path}", get(api_not_found))
        .with_state(state)
}
