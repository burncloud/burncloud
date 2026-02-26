use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::{get, post},
    Router,
};
use burncloud_common::types::Channel;
use burncloud_database_models::ChannelModel;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Pagination query parameters
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    /// Number of items per page (default: 20, max: 100)
    #[serde(default = "default_limit")]
    pub limit: i32,
    /// Offset from the start (default: 0)
    #[serde(default)]
    pub offset: i32,
}

fn default_limit() -> i32 {
    20
}

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
    pub param_override: Option<String>,
    pub header_override: Option<String>,
    pub api_version: Option<String>,
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
            param_override: self.param_override,
            header_override: self.header_override,
            remark: None,
            api_version: self.api_version,
            pricing_region: None,
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
    Query(params): Query<PaginationParams>,
) -> Json<Value> {
    // Clamp limit to reasonable bounds
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);

    match ChannelModel::list(&state.db, limit, offset).await {
        Ok(channels) => Json(json!({
            "success": true,
            "data": channels,
            "pagination": {
                "limit": limit,
                "offset": offset
            }
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
