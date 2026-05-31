use crate::AppState;
pub mod security;

use axum::{middleware, Router};

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
        .layer(middleware::from_fn(crate::auth_middleware))
        .with_state(state);

    // Merge public and protected routes
    public_routes.merge(protected_routes)
}
