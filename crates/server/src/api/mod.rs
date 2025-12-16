use crate::AppState;
use axum::Router;

pub mod auth;
pub mod channel;
pub mod group;
pub mod log;
pub mod monitor;
pub mod token;
pub mod user;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .merge(auth::routes())
        .merge(channel::routes())
        .merge(group::routes())
        .merge(token::routes())
        .merge(log::routes())
        .merge(monitor::routes())
        .merge(user::routes())
        .with_state(state)
}
