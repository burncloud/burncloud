use crate::AppState;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use burncloud_database_router::{RouterDatabase, RouterUpstream};
use serde::Serialize;

#[derive(Serialize)]
struct ApiError {
    error: String,
}

fn upstream_err(e: impl ToString) -> impl IntoResponse {
    Json(ApiError {
        error: e.to_string(),
    })
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/console/api/upstreams",
            post(create_upstream).get(list_upstreams),
        )
        .route(
            "/console/api/upstreams/{id}",
            get(get_upstream).put(update_upstream).delete(delete_upstream),
        )
}

async fn list_upstreams(State(state): State<AppState>) -> impl IntoResponse {
    match RouterDatabase::get_all_upstreams(&state.db).await {
        Ok(upstreams) => Json(upstreams).into_response(),
        Err(e) => upstream_err(e).into_response(),
    }
}

async fn create_upstream(
    State(state): State<AppState>,
    Json(payload): Json<RouterUpstream>,
) -> impl IntoResponse {
    match RouterDatabase::create_upstream(&state.db, &payload).await {
        Ok(_) => Json(payload).into_response(),
        Err(e) => upstream_err(e).into_response(),
    }
}

async fn get_upstream(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match RouterDatabase::get_upstream(&state.db, &id).await {
        Ok(Some(u)) => Json(u).into_response(),
        Ok(None) => upstream_err("Not Found").into_response(),
        Err(e) => upstream_err(e).into_response(),
    }
}

async fn update_upstream(
    State(state): State<AppState>,
    Path(_id): Path<String>,
    Json(payload): Json<RouterUpstream>,
) -> impl IntoResponse {
    match RouterDatabase::update_upstream(&state.db, &payload).await {
        Ok(_) => Json(payload).into_response(),
        Err(e) => upstream_err(e).into_response(),
    }
}

async fn delete_upstream(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match RouterDatabase::delete_upstream(&state.db, &id).await {
        Ok(_) => Json(serde_json::json!({"status": "deleted", "id": id})).into_response(),
        Err(e) => upstream_err(e).into_response(),
    }
}
