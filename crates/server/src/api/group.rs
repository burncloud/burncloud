use axum::{
    response::Json,
    routing::post,
    Router,
};
use serde_json::{json, Value};

pub fn routes() -> Router {
    Router::new()
        .route("/groups", post(create_group).get(list_groups))
}

async fn list_groups() -> Json<Value> {
    Json(json!([{ "id": "g1", "name": "Demo Group" }]))
}

async fn create_group() -> Json<Value> {
    Json(json!({ "status": "created" }))
}
