use axum::Router;

pub mod channel;
pub mod group;

pub fn routes() -> Router {
    Router::new()
        .merge(channel::routes())
        .merge(group::routes())
}
