use crate::AppState;
use axum::{extract::State, response::Json, routing::get, Router};
use serde_json::{json, Value};

pub fn routes() -> Router<AppState> {
    Router::new().route("/console/api/monitor", get(get_system_metrics))
}

async fn get_system_metrics(State(state): State<AppState>) -> Json<Value> {
    match state.monitor.get_metrics().await {
        Ok(metrics) => Json(json!({
            "success": true,
            "data": metrics
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e.to_string()
        })),
    }
}
