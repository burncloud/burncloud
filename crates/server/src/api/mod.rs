use axum::Router;
use crate::AppState;

pub mod channel;
pub mod group;
pub mod token;
pub mod log;
pub mod monitor;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .merge(channel::routes())
        .merge(group::routes())
        .merge(token::routes())
        .merge(log::routes())
        .merge(monitor::routes())
        .with_state(state)
}
