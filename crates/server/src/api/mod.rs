use crate::AppState;
pub mod security;

use axum::{http::StatusCode, middleware, response::IntoResponse, routing::get, Router};

pub mod auth;
pub mod billing;
pub mod cache;
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
    // Public routes - no authentication required
    // These are auth endpoints that must be accessible without a token
    let public_routes = Router::new()
        .merge(auth::public_routes())
        .with_state(state.clone());

    // Protected routes - authentication required
    // All /console/api/* endpoints (except public auth routes) require a valid JWT
    let protected_routes = Router::new()
        .merge(auth::protected_routes())
        .merge(billing::routes())
        .merge(channel::routes())
        .merge(token::routes())
        .merge(log::routes())
        .merge(monitor::routes())
        .merge(user::routes())
        .merge(security::security_routes())
        .merge(openapi::routes())
        .merge(cache::routes())
        // Catch-all for any unmatched /console/api/* paths
        // This prevents LiveView from returning HTML for non-existent API endpoints
        .route("/console/api/{*path}", get(api_not_found))
        .layer(middleware::from_fn(crate::auth_middleware))
        .with_state(state);

    // Merge public and protected routes
    public_routes.merge(protected_routes)
}
