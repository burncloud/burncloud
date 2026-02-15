use crate::AppState;
use axum::{
    extract::{Path, State},
    response::Json,
    routing::{get, post},
    Router,
};
use burncloud_database_router::{DbToken, RouterDatabase};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub struct CreateTokenRequest {
    pub user_id: String,
    pub quota_limit: Option<i64>,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateTokenRequest {
    pub status: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/console/api/tokens", post(create_token).get(list_tokens))
        .route(
            "/console/api/tokens/{token}",
            get(delete_token).delete(delete_token).put(update_token),
        )
}

async fn list_tokens(State(state): State<AppState>) -> Json<Value> {
    log::info!("[API] list_tokens request received");
    match RouterDatabase::list_tokens(&state.db).await {
        Ok(tokens) => {
            log::info!("[API] list_tokens success: found {} tokens", tokens.len());
            Json(json!(tokens))
        }
        Err(e) => {
            log::error!("[API] list_tokens error: {}", e);
            Json(json!({ "error": e.to_string() }))
        }
    }
}

async fn create_token(
    State(state): State<AppState>,
    Json(payload): Json<CreateTokenRequest>,
) -> Json<Value> {
    log::info!(
        "[API] create_token request: user_id={}, quota={:?}",
        payload.user_id,
        payload.quota_limit
    );

    // Generate a random sk- key
    let token = format!("sk-burncloud-{}", Uuid::new_v4());

    let db_token = DbToken {
        token: token.clone(),
        user_id: payload.user_id,
        status: "active".to_string(),
        quota_limit: payload.quota_limit.unwrap_or(-1),
        used_quota: 0,
        accessed_time: 0,
        expired_time: -1,
    };

    match RouterDatabase::create_token(&state.db, &db_token).await {
        Ok(_) => {
            log::info!("[API] create_token success: {}", token);
            Json(json!({ "status": "created", "token": token }))
        }
        Err(e) => {
            log::error!("[API] create_token error: {}", e);
            Json(json!({ "error": e.to_string() }))
        }
    }
}

async fn update_token(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(payload): Json<UpdateTokenRequest>,
) -> Json<Value> {
    log::info!(
        "[API] update_token request: {} -> {}",
        token,
        payload.status
    );
    match RouterDatabase::update_token_status(&state.db, &token, &payload.status).await {
        Ok(_) => {
            log::info!("[API] update_token success");
            Json(json!({ "status": "updated", "token": token }))
        }
        Err(e) => {
            log::error!("[API] update_token error: {}", e);
            Json(json!({ "error": e.to_string() }))
        }
    }
}

async fn delete_token(State(state): State<AppState>, Path(token): Path<String>) -> Json<Value> {
    log::info!("[API] delete_token request: {}", token);
    match RouterDatabase::delete_token(&state.db, &token).await {
        Ok(_) => {
            log::info!("[API] delete_token success");
            Json(json!({ "status": "deleted", "token": token }))
        }
        Err(e) => {
            log::error!("[API] delete_token error: {}", e);
            Json(json!({ "error": e.to_string() }))
        }
    }
}
