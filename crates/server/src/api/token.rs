use crate::api::auth::{is_admin, Claims};
use crate::AppState;
use axum::{
    extract::{Extension, Json, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use burncloud_service_token::{RouterToken, TokenService};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::response::{err, err_status, ok};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateTokenRequest {
    pub user_id: String,
    /// Spend limit in nanodollars. `-1` means unlimited.
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

/// A management-plane representation that never returns the bearer secret.
/// The full token is only returned once by create/rotate operations.
#[derive(Debug, Serialize)]
struct TokenSummary {
    token_hint: String,
    user_id: String,
    status: String,
    quota_limit: i64,
    used_quota: i64,
    expired_time: i64,
    accessed_time: i64,
    key_version: i32,
    old_key_expires_at: i64,
    ip_whitelist: Option<String>,
    key_prefix: String,
    created_at: i64,
    last_rotated_at: i64,
}

fn token_hint(token: &RouterToken) -> String {
    let suffix: String = token.token.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
    format!("{}…{}", token.key_prefix, suffix)
}

impl From<RouterToken> for TokenSummary {
    fn from(token: RouterToken) -> Self {
        let hint = token_hint(&token);
        Self {
            token_hint: hint,
            user_id: token.user_id,
            status: token.status,
            quota_limit: token.quota_limit,
            used_quota: token.used_quota,
            expired_time: token.expired_time,
            accessed_time: token.accessed_time,
            key_version: token.key_version,
            old_key_expires_at: token.old_key_expires_at,
            ip_whitelist: token.ip_whitelist,
            key_prefix: token.key_prefix,
            created_at: token.created_at,
            last_rotated_at: token.last_rotated_at,
        }
    }
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

async fn principal_is_admin(state: &AppState, claims: &Claims) -> Result<bool, Response> {
    is_admin(state, claims)
        .await
        .map_err(|status| err_status(status, "Failed to authorize request").into_response())
}

async fn authorized_token(
    state: &AppState,
    claims: &Claims,
    token: &str,
) -> Result<RouterToken, Response> {
    let admin = principal_is_admin(state, claims).await?;
    let tokens = TokenService::list(&state.db).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to load API token for authorization");
        err_status(StatusCode::INTERNAL_SERVER_ERROR, "Failed to load API token").into_response()
    })?;

    let Some(record) = tokens.into_iter().find(|record| record.token == token) else {
        return Err(err_status(StatusCode::NOT_FOUND, "Token not found").into_response());
    };

    if admin || record.user_id == claims.sub {
        Ok(record)
    } else {
        Err(err_status(StatusCode::FORBIDDEN, "Token access denied").into_response())
    }
}

#[tracing::instrument(skip_all)]
async fn list_tokens(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    let admin = match principal_is_admin(&state, &claims).await {
        Ok(value) => value,
        Err(response) => return response,
    };

    match TokenService::list(&state.db).await {
        Ok(tokens) => {
            let summaries: Vec<TokenSummary> = tokens
                .into_iter()
                .filter(|token| admin || token.user_id == claims.sub)
                .map(TokenSummary::from)
                .collect();
            ok(summaries).into_response()
        }
        Err(e) => {
            tracing::error!("[API] list_tokens error: {}", e);
            err(e).into_response()
        }
    }
}

#[tracing::instrument(skip_all, fields(user_id = %payload.user_id))]
async fn create_token(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateTokenRequest>,
) -> impl IntoResponse {
    let admin = match principal_is_admin(&state, &claims).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if !admin && payload.user_id != claims.sub {
        return err_status(
            StatusCode::FORBIDDEN,
            "Users may only create API tokens for themselves",
        )
        .into_response();
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let token_str = format!("bc_live_{}", Uuid::new_v4());
    let user_id = payload.user_id;

    let db_token = RouterToken {
        token: token_str.clone(),
        user_id: user_id.clone(),
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
            tracing::info!(user_id, "API token created");
            // Creation is the single disclosure point for the bearer secret.
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

#[tracing::instrument(skip_all)]
async fn get_token(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(token): Path<String>,
) -> impl IntoResponse {
    match authorized_token(&state, &claims, &token).await {
        Ok(record) => ok(TokenSummary::from(record)).into_response(),
        Err(response) => response,
    }
}

#[tracing::instrument(skip_all, fields(status = %payload.status))]
async fn update_token(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(token): Path<String>,
    Json(payload): Json<UpdateTokenRequest>,
) -> impl IntoResponse {
    if let Err(response) = authorized_token(&state, &claims, &token).await {
        return response;
    }

    match TokenService::update_status(&state.db, &token, &payload.status).await {
        Ok(_) => ok(serde_json::json!({ "status": "updated" })).into_response(),
        Err(e) => {
            tracing::error!("[API] update_token error: {}", e);
            err(e).into_response()
        }
    }
}

#[tracing::instrument(skip_all)]
async fn delete_token(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(token): Path<String>,
) -> impl IntoResponse {
    if let Err(response) = authorized_token(&state, &claims, &token).await {
        return response;
    }

    match TokenService::delete(&state.db, &token).await {
        Ok(_) => ok(serde_json::json!({ "status": "deleted" })).into_response(),
        Err(e) => {
            tracing::error!("[API] delete_token error: {}", e);
            err(e).into_response()
        }
    }
}

#[tracing::instrument(skip_all)]
async fn rotate_token(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(token): Path<String>,
    Json(payload): Json<RotateTokenRequest>,
) -> impl IntoResponse {
    if let Err(response) = authorized_token(&state, &claims, &token).await {
        return response;
    }

    match TokenService::rotate(
        &state.db,
        &token,
        payload.transition_period_hours,
        payload.revoke_old,
    )
    .await
    {
        Ok(result) => {
            tracing::info!(new_version = result.key_version, "API token rotated");
            // Rotation is the single disclosure point for the new bearer secret.
            ok(result).into_response()
        }
        Err(e) => {
            tracing::error!("[API] rotate_token error: {}", e);
            err(e).into_response()
        }
    }
}

#[tracing::instrument(skip_all)]
async fn revoke_old_key(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(token): Path<String>,
) -> impl IntoResponse {
    if let Err(response) = authorized_token(&state, &claims, &token).await {
        return response;
    }

    match TokenService::revoke_old_key(&state.db, &token).await {
        Ok(true) => ok(serde_json::json!({ "status": "revoked" })).into_response(),
        Ok(false) => err_status(StatusCode::NOT_FOUND, "Token not found").into_response(),
        Err(e) => {
            tracing::error!("[API] revoke_old_key error: {}", e);
            err(e).into_response()
        }
    }
}

#[tracing::instrument(skip_all)]
async fn set_ip_whitelist(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(token): Path<String>,
    Json(payload): Json<SetIpWhitelistRequest>,
) -> impl IntoResponse {
    if let Err(response) = authorized_token(&state, &claims, &token).await {
        return response;
    }

    match TokenService::set_ip_whitelist(&state.db, &token, &payload.ip_whitelist).await {
        Ok(true) => ok(serde_json::json!({ "status": "updated" })).into_response(),
        Ok(false) => err_status(StatusCode::NOT_FOUND, "Token not found").into_response(),
        Err(e) => {
            tracing::error!("[API] set_ip_whitelist error: {}", e);
            err(e).into_response()
        }
    }
}
