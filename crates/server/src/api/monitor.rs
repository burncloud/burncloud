use crate::api::response::{err, ok};
use crate::AppState;
use axum::{extract::State, response::IntoResponse, routing::get, Router};

pub fn routes() -> Router<AppState> {
    Router::new().route("/console/api/monitor", get(get_system_metrics))
}

async fn get_system_metrics(State(state): State<AppState>) -> impl IntoResponse {
    match state.monitor.get_metrics().await {
        Ok(metrics) => ok(metrics).into_response(),
        Err(e) => err(e).into_response(),
    }
}
