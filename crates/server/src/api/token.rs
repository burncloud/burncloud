use crate::AppState;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use burncloud_service_token::{RouterToken, TokenService};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::response::{err, ok};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateTokenRequest {
    pub user_id: String,
    pub quota_limit: Option<i64>,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateTokenRequest {
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RotateTokenRequest {
    /// Hours the old key remains valid (0 = use default 24 hours)
    #[serde(default)]
    pub transition_period_hours: i32,
    /// Whether to immediately revoke the old key
    #[serde(default)]
    pub revoke_old: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SetIpWhitelistRequest {
    pub ip_whitelist: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/console/api/tokens", post(create_token).get(list_tokens))
        .route(
            "/console/api/tokens/{token}",
            get(get_token).delete(delete_token).put(update_token),
        )
        .route("/console/api/tokens/{token}/rotate", post(rotate_token))
        .route(
            "/console/api/tokens/{token}/revoke-old",
            post(revoke_old_key),
        )
        .route(
            "/console/api/tokens/{token}/ip-whitelist",
            post(set_ip_whitelist),
        )
}

#[tracing::instrument(skip_all)]
async fn list_tokens(State(state): State<AppState>) -> impl IntoResponse {
    tracing::info!("[API] list_tokens request received");
    match TokenService::list(&state.db).await {
        Ok(tokens) => {
            tracing::info!("[API] list_tokens success: found {} tokens", tokens.len());
            ok(tokens).into_response()
        }
        Err(e) => {
            tracing::error!("[API] list_tokens error: {}", e);
            err(e).into_response()
        }
    }
}

#[tracing::instrument(skip(state), fields(user_id = %payload.user_id))]
async fn create_token(
    State(state): State<AppState>,
    Json(payload): Json<CreateTokenRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "[API] create_token request: user_id={}, quota={:?}",
        payload.user_id,
        payload.quota_limit
    );

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let token_str = format!("bc_live_{}", Uuid::new_v4());

    let db_token = RouterToken {
        token: token_str.clone(),
        user_id: payload.user_id,
        status: "active".to_string(),
        quota_limit: payload.quota_limit.unwrap_or(-1),
        used_quota: 0,
        accessed_time: 0,
        expired_time: -1,
        key_version: 1,
        old_key_hash: None,
        old_key_expires_at: 0,
        ip_whitelist: None,
        key_prefix: "bc_live_".to_string(),
        created_at: now,
        last_rotated_at: 0,
    };

    match TokenService::create(&state.db, &db_token).await {
        Ok(_) => {
            tracing::info!("[API] create_token success: {}", token_str);
            ok(serde_json::json!({
                "status": "created",
                "token": token_str
            }))
            .into_response()
        }
        Err(e) => {
            tracing::error!("[API] create_token error: {}", e);
            err(e).into_response()
        }
    }
}

#[tracing::instrument(skip(state))]
async fn get_token(State(state): State<AppState>, Path(token): Path<String>) -> impl IntoResponse {
    tracing::info!("[API] get_token request: {}", token);
    match TokenService::validate(&state.db, &token).await {
        Ok(Some(t)) => {
            tracing::info!("[API] get_token success");
            ok(t).into_response()
        }
        Ok(None) => {
            tracing::info!("[API] get_token: token not found");
            err("Token not found").into_response()
        }
        Err(e) => {
            tracing::error!("[API] get_token error: {}", e);
            err(e).into_response()
        }
    }
}

#[tracing::instrument(skip(state, payload), fields(status = %payload.status))]
async fn update_token(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(payload): Json<UpdateTokenRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "[API] update_token request: {} -> {}",
        token,
        payload.status
    );
    match TokenService::update_status(&state.db, &token, &payload.status).await {
        Ok(_) => {
            tracing::info!("[API] update_token success");
            ok(serde_json::json!({
                "status": "updated",
                "token": token
            }))
            .into_response()
        }
        Err(e) => {
            tracing::error!("[API] update_token error: {}", e);
            err(e).into_response()
        }
    }
}

#[tracing::instrument(skip(state))]
async fn delete_token(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> impl IntoResponse {
    tracing::info!("[API] delete_token request: {}", token);
    match TokenService::delete(&state.db, &token).await {
        Ok(_) => {
            tracing::info!("[API] delete_token success");
            ok(serde_json::json!({
                "status": "deleted",
                "token": token
            }))
            .into_response()
        }
        Err(e) => {
            tracing::error!("[API] delete_token error: {}", e);
            err(e).into_response()
        }
    }
}

#[tracing::instrument(skip(state))]
async fn rotate_token(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(payload): Json<RotateTokenRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "[API] rotate_token request: token={}, transition_hours={}, revoke_old={}",
        token,
        payload.transition_period_hours,
        payload.revoke_old
    );

    match TokenService::rotate(
        &state.db,
        &token,
        payload.transition_period_hours,
        payload.revoke_old,
    )
    .await
    {
        Ok(result) => {
            tracing::info!(
                "[API] rotate_token success: new_version={}",
                result.key_version
            );
            ok(result).into_response()
        }
        Err(e) => {
            tracing::error!("[API] rotate_token error: {}", e);
            err(e).into_response()
        }
    }
}

#[tracing::instrument(skip(state))]
async fn revoke_old_key(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> impl IntoResponse {
    tracing::info!("[API] revoke_old_key request: {}", token);

    match TokenService::revoke_old_key(&state.db, &token).await {
        Ok(true) => {
            tracing::info!("[API] revoke_old_key success");
            ok(serde_json::json!({
                "status": "revoked",
                "token": token
            }))
            .into_response()
        }
        Ok(false) => {
            tracing::info!("[API] revoke_old_key: token not found");
            err("Token not found").into_response()
        }
        Err(e) => {
            tracing::error!("[API] revoke_old_key error: {}", e);
            err(e).into_response()
        }
    }
}

#[tracing::instrument(skip(state))]
async fn set_ip_whitelist(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(payload): Json<SetIpWhitelistRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "[API] set_ip_whitelist request: token={}, whitelist={}",
        token,
        payload.ip_whitelist
    );

    match TokenService::set_ip_whitelist(&state.db, &token, &payload.ip_whitelist).await {
        Ok(true) => {
            tracing::info!("[API] set_ip_whitelist success");
            ok(serde_json::json!({
                "status": "updated",
                "token": token
            }))
            .into_response()
        }
        Ok(false) => {
            tracing::info!("[API] set_ip_whitelist: token not found");
            err("Token not found").into_response()
        }
        Err(e) => {
            tracing::error!("[API] set_ip_whitelist error: {}", e);
            err(e).into_response()
        }
    }
}
