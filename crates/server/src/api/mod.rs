use crate::AppState;
pub mod security;

use axum::{
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::get,
    Router,
};

pub mod auth;
pub mod billing;
pub mod cache;
pub mod channel;
pub mod log;
pub mod monitor;
pub mod openapi;
pub(crate) mod registration;
#[cfg(test)]
mod registration_tests;
pub mod response;
pub mod token;
pub mod user;

/// Fallback handler for unmatched /console/api/* requests
/// Returns 404 instead of being caught by LiveView's catch-all
async fn api_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "API endpoint not found")
}

pub fn routes(state: AppState) -> Router {
    // Public routes - no authentication required.
    let public_routes = Router::new()
        .merge(auth::public_routes())
        .with_state(state.clone());

    // Administrator-only management surfaces. Authentication runs on the
    // outer protected router; this inner layer performs authorization.
    let admin_routes = Router::new()
        .merge(channel::routes())
        .merge(log::routes())
        .merge(monitor::routes())
        .merge(security::security_routes())
        .merge(cache::routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::admin_middleware,
        ));

    // Authenticated self-service routes. Token handlers perform resource-level
    // owner/admin authorization because users are allowed to manage their own
    // API credentials while administrators may manage all credentials.
    let protected_routes = Router::new()
        .merge(auth::protected_routes())
        .merge(billing::routes())
        .merge(token::routes())
        .merge(user::routes())
        .merge(openapi::routes())
        .merge(admin_routes)
        // Catch-all for any unmatched /console/api/* paths. This prevents
        // LiveView from returning HTML for non-existent API endpoints.
        .route("/console/api/{*path}", get(api_not_found))
        .layer(middleware::from_fn(crate::auth_middleware))
        .with_state(state);

    public_routes.merge(protected_routes)
}