use crate::api::response::{err, ok};
use crate::AppState;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use burncloud_service_upstream::{RouterUpstream, UpstreamService};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/console/api/upstreams",
            post(create_upstream).get(list_upstreams),
        )
        .route(
            "/console/api/upstreams/{id}",
            get(get_upstream)
                .put(update_upstream)
                .delete(delete_upstream),
        )
}

async fn list_upstreams(State(state): State<AppState>) -> impl IntoResponse {
    match UpstreamService::get_all(&state.db).await {
        Ok(upstreams) => ok(upstreams).into_response(),
        Err(e) => err(e.to_string()).into_response(),
    }
}

async fn create_upstream(
    State(state): State<AppState>,
    Json(payload): Json<RouterUpstream>,
) -> impl IntoResponse {
    match UpstreamService::create(&state.db, &payload).await {
        Ok(_) => ok(payload).into_response(),
        Err(e) => err(e.to_string()).into_response(),
    }
}

async fn get_upstream(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match UpstreamService::get(&state.db, &id).await {
        Ok(Some(u)) => ok(u).into_response(),
        Ok(None) => err("Not Found").into_response(),
        Err(e) => err(e.to_string()).into_response(),
    }
}

async fn update_upstream(
    State(state): State<AppState>,
    Path(_id): Path<String>,
    Json(payload): Json<RouterUpstream>,
) -> impl IntoResponse {
    match UpstreamService::update(&state.db, &payload).await {
        Ok(_) => ok(payload).into_response(),
        Err(e) => err(e.to_string()).into_response(),
    }
}

async fn delete_upstream(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match UpstreamService::delete(&state.db, &id).await {
        Ok(_) => ok(serde_json::json!({"status": "deleted", "id": id})).into_response(),
        Err(e) => err(e.to_string()).into_response(),
    }
}
