use crate::AppState;
pub mod security;

use axum::Router;

pub mod auth;
pub mod billing;
pub mod channel;
pub mod group;
pub mod log;
pub mod monitor;
pub mod openapi;
pub mod response;
pub mod token;
pub mod user;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .merge(auth::routes())
        .merge(billing::routes())
        .merge(channel::routes())
        .merge(group::routes())
        .merge(token::routes())
        .merge(log::routes())
        .merge(monitor::routes())
        .merge(user::routes())
        .merge(security::security_routes())
        .merge(openapi::routes())
        .with_state(state)
}
