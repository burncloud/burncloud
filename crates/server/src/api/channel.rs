use axum::{
    extract::Path,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// TODO: Import DbUpstream properly. For now, define a DTO.
#[derive(Deserialize, Serialize)]
pub struct ChannelDto {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub match_path: String,
    pub auth_type: String,
    pub priority: i32,
}

pub fn routes() -> Router {
    Router::new()
        .route("/channels", post(create_channel).get(list_channels))
        .route("/channels/:id", get(get_channel).put(update_channel).delete(delete_channel))
}

async fn list_channels() -> Json<Value> {
    // Placeholder
    Json(json!([{ "id": "demo", "name": "Demo Channel" }]))
}

async fn create_channel(Json(payload): Json<ChannelDto>) -> Json<Value> {
    Json(json!({ "status": "created", "id": payload.id }))
}

async fn get_channel(Path(id): Path<String>) -> Json<Value> {
    Json(json!({ "id": id, "name": "Mock Channel" }))
}

async fn update_channel(Path(id): Path<String>, Json(_payload): Json<ChannelDto>) -> Json<Value> {
    Json(json!({ "status": "updated", "id": id }))
}

async fn delete_channel(Path(id): Path<String>) -> Json<Value> {
    Json(json!({ "status": "deleted", "id": id }))
}
