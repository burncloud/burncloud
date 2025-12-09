use crate::AppState;
use axum::{
    extract::{Path, State},
    response::Json,
    routing::{get, post},
    Router,
};
use burncloud_common::types::Channel;
use burncloud_database_models::ChannelModel;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize, Serialize, Clone)]
pub struct ChannelDto {
    pub id: Option<i32>,
    #[serde(rename = "type")]
    pub type_: i32,
    pub key: String,
    pub name: String,
    pub base_url: Option<String>,
    pub models: String,
    pub group: String,
    pub weight: i32,
    pub priority: i64,
}

impl ChannelDto {
    fn into_channel(self) -> Channel {
        Channel {
            id: self.id.unwrap_or(0),
            type_: self.type_,
            key: self.key,
            status: 1,
            name: self.name,
            weight: self.weight,
            created_time: None,
            test_time: None,     // Initial state
            response_time: None, // Initial state
            base_url: self.base_url,
            models: self.models,
            group: self.group,
            used_quota: 0,
            model_mapping: None,
            priority: self.priority,
            auto_ban: 1,
            other_info: None,
            tag: None,
            setting: None,
            param_override: None,
            header_override: None,
            remark: None,
        }
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/console/api/channel",
            post(create_channel).put(update_channel).get(list_channels),
        )
        .route(
            "/console/api/channel/{id}",
            get(get_channel).delete(delete_channel),
        )
}

async fn list_channels(
    State(state): State<AppState>,
    // TODO: Add pagination params
) -> Json<Value> {
    match ChannelModel::list(&state.db, 100, 0).await {
        Ok(channels) => Json(json!({
            "success": true,
            "data": channels
        })),
        Err(e) => Json(json!({ "success": false, "message": e.to_string() })),
    }
}

async fn create_channel(
    State(state): State<AppState>,
    Json(payload): Json<ChannelDto>,
) -> Json<Value> {
    let mut channel = payload.into_channel();
    match ChannelModel::create(&state.db, &mut channel).await {
        Ok(id) => Json(json!({
            "success": true,
            "message": "channel created",
            "data": { "id": id }
        })),
        Err(e) => Json(json!({
            "success": false,
            "message": e.to_string()
        })),
    }
}

async fn update_channel(
    State(state): State<AppState>,
    Json(payload): Json<ChannelDto>,
) -> Json<Value> {
    let channel = payload.into_channel();
    if channel.id == 0 {
        return Json(json!({ "success": false, "message": "id is required" }));
    }
    match ChannelModel::update(&state.db, &channel).await {
        Ok(_) => Json(json!({
            "success": true,
            "message": "channel updated",
            "data": channel
        })),
        Err(e) => Json(json!({
            "success": false,
            "message": e.to_string()
        })),
    }
}

async fn delete_channel(State(state): State<AppState>, Path(id): Path<i32>) -> Json<Value> {
    match ChannelModel::delete(&state.db, id).await {
        Ok(_) => Json(json!({
            "success": true,
            "message": "channel deleted"
        })),
        Err(e) => Json(json!({
            "success": false,
            "message": e.to_string()
        })),
    }
}

async fn get_channel(State(state): State<AppState>, Path(id): Path<i32>) -> Json<Value> {
    match ChannelModel::get_by_id(&state.db, id).await {
        Ok(Some(c)) => Json(json!({
            "success": true,
            "data": c
        })),
        Ok(None) => Json(json!({ "success": false, "message": "channel not found" })),
        Err(e) => Json(json!({ "success": false, "message": e.to_string() })),
    }
}
