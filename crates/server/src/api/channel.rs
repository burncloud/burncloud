use axum::{
    extract::{Path, State},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::AppState;
use burncloud_database_router::{RouterDatabase, DbUpstream};

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

impl From<ChannelDto> for DbUpstream {
    fn from(dto: ChannelDto) -> Self {
        DbUpstream {
            id: dto.id,
            name: dto.name,
            base_url: dto.base_url,
            api_key: dto.api_key,
            match_path: dto.match_path,
            auth_type: dto.auth_type,
            priority: dto.priority,
        }
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/channels", post(create_channel).get(list_channels))
        .route("/channels/{id}", get(get_channel).put(update_channel).delete(delete_channel))
}

async fn list_channels(State(state): State<AppState>) -> Json<Value> {
    match RouterDatabase::get_all_upstreams(&state.db).await {
        Ok(channels) => Json(json!(channels)),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn create_channel(State(state): State<AppState>, Json(payload): Json<ChannelDto>) -> Json<Value> {
    let upstream: DbUpstream = payload.into();
    match RouterDatabase::create_upstream(&state.db, &upstream).await {
        Ok(_) => Json(json!({ "status": "created", "id": upstream.id })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn get_channel(State(state): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    match RouterDatabase::get_upstream(&state.db, &id).await {
        Ok(Some(u)) => Json(json!(u)),
        Ok(None) => Json(json!({ "error": "Not Found" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn update_channel(State(state): State<AppState>, Path(id): Path<String>, Json(mut payload): Json<ChannelDto>) -> Json<Value> {
    payload.id = id.clone(); // Ensure ID matches path
    let upstream: DbUpstream = payload.into();
    match RouterDatabase::update_upstream(&state.db, &upstream).await {
        Ok(_) => Json(json!({ "status": "updated", "id": id })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn delete_channel(State(state): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    match RouterDatabase::delete_upstream(&state.db, &id).await {
        Ok(_) => Json(json!({ "status": "deleted", "id": id })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}
