use axum::Router;
use crate::AppState;

pub mod channel;
pub mod group;
pub mod token;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .merge(channel::routes())
        .merge(group::routes())
        .merge(token::routes())
        .with_state(state)
}
