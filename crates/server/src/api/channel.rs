use crate::api::response::{err, ok};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use burncloud_service_channel::{Channel, ChannelService};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_limit")]
    pub limit: i32,
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
    // L2 Shaper config (issue #151). Omit to leave channel fail-open.
    #[serde(default)]
    pub rpm_cap: Option<i32>,
    #[serde(default)]
    pub tpm_cap: Option<i64>,
    #[serde(default)]
    pub reservation_green: Option<f64>,
    #[serde(default)]
    pub reservation_yellow: Option<f64>,
    #[serde(default)]
    pub reservation_red: Option<f64>,
}

#[derive(Serialize)]
struct ChannelListData {
    channels: Vec<Channel>,
    pagination: PaginationInfo,
}

#[derive(Serialize)]
struct PaginationInfo {
    limit: i32,
    offset: i32,
}

#[derive(Serialize)]
struct ChannelCreated {
    id: i32,
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
            test_time: None,
            response_time: None,
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
            rpm_cap: self.rpm_cap,
            tpm_cap: self.tpm_cap,
            reservation_green: self.reservation_green,
            reservation_yellow: self.reservation_yellow,
            reservation_red: self.reservation_red,
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
) -> impl IntoResponse {
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);

    match ChannelService::list(&state.db, limit, offset).await {
        Ok(channels) => ok(ChannelListData {
            channels,
            pagination: PaginationInfo { limit, offset },
        })
        .into_response(),
        Err(e) => err(e).into_response(),
    }
}

async fn create_channel(
    State(state): State<AppState>,
    axum::extract::Json(payload): axum::extract::Json<ChannelDto>,
) -> impl IntoResponse {
    let mut channel = payload.into_channel();
    match ChannelService::create(&state.db, &mut channel).await {
        Ok(id) => ok(ChannelCreated { id }).into_response(),
        Err(e) => err(e).into_response(),
    }
}

async fn update_channel(
    State(state): State<AppState>,
    axum::extract::Json(payload): axum::extract::Json<ChannelDto>,
) -> impl IntoResponse {
    let channel = payload.into_channel();
    if channel.id == 0 {
        return err("id is required").into_response();
    }
    match ChannelService::update(&state.db, &channel).await {
        Ok(_) => ok(channel).into_response(),
        Err(e) => err(e).into_response(),
    }
}

async fn delete_channel(State(state): State<AppState>, Path(id): Path<i32>) -> impl IntoResponse {
    match ChannelService::delete(&state.db, id).await {
        Ok(_) => ok(()).into_response(),
        Err(e) => err(e).into_response(),
    }
}

async fn get_channel(State(state): State<AppState>, Path(id): Path<i32>) -> impl IntoResponse {
    match ChannelService::get_by_id(&state.db, id).await {
        Ok(Some(c)) => ok(c).into_response(),
        Ok(None) => err("channel not found").into_response(),
        Err(e) => err(e).into_response(),
    }
}
