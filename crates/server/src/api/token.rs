use crate::api::auth::{is_admin, Claims};
use crate::AppState;
use axum::{
    body::Body,
    extract::{Extension, Json, Path, State},
    http::{header, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use burncloud_service_token::{RouterToken, TokenService};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
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

#[derive(Debug, Deserialize, Serialize)]
struct PlaygroundMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct PlaygroundChatRequest {
    /// Opaque management-plane token reference returned by list_tokens.
    token_ref: String,
    model: String,
    messages: Vec<PlaygroundMessage>,
    temperature: f64,
    max_tokens: i64,
}

/// A management-plane representation that never returns the bearer secret.
/// `token` is intentionally an opaque management reference for backwards
/// compatibility with existing console DTOs; it is not accepted as a data-plane
/// bearer credential. The human-readable `token_hint` is safe to display.
#[derive(Debug, Serialize)]
struct TokenSummary {
    token: String,
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
    let suffix: String = token
        .token
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{}…{}", token.key_prefix, suffix)
}

/// Stable, non-bearer management identifier for a token record.
///
/// The source token is generated with high entropy. Returning its SHA-256 digest
/// lets the authenticated management plane address the record without disclosing
/// a credential that can be used against `/v1/*`.
fn token_management_id(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    format!("tok_{digest:x}")
}

impl From<RouterToken> for TokenSummary {
    fn from(token: RouterToken) -> Self {
        let management_id = token_management_id(&token.token);
        let hint = token_hint(&token);
        Self {
            token: management_id,
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
            "/console/api/tokens/{token_ref}",
            get(get_token).delete(delete_token).put(update_token),
        )
        .route(
            "/console/api/tokens/{token_ref}/rotate",
            post(rotate_token),
        )
        .route(
            "/console/api/tokens/{token_ref}/revoke-old",
            post(revoke_old_key),
        )
        .route(
            "/console/api/tokens/{token_ref}/ip-whitelist",
            post(set_ip_whitelist),
        )
        .route("/console/api/playground/chat", post(playground_chat))
}

async fn principal_is_admin(state: &AppState, claims: &Claims) -> Result<bool, Response> {
    is_admin(state, claims)
        .await
        .map_err(|status| err_status(status, "Failed to authorize request").into_response())
}

/// Resolve either the opaque management reference or, for backwards-compatible
/// authenticated management calls, the exact bearer token. The latter is never
/// returned by list/get responses.
pub(crate) async fn authorized_token(
    state: &AppState,
    claims: &Claims,
    token_ref: &str,
) -> Result<RouterToken, Response> {
    let admin = principal_is_admin(state, claims).await?;
    let tokens = TokenService::list(&state.db).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to load API token for authorization");
        err_status(StatusCode::INTERNAL_SERVER_ERROR, "Failed to load API token").into_response()
    })?;

    let Some(record) = tokens
        .into_iter()
        .find(|record| record.token == token_ref || token_management_id(&record.token) == token_ref)
    else {
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
    Path(token_ref): Path<String>,
) -> impl IntoResponse {
    match authorized_token(&state, &claims, &token_ref).await {
        Ok(record) => ok(TokenSummary::from(record)).into_response(),
        Err(response) => response,
    }
}

#[tracing::instrument(skip_all, fields(status = %payload.status))]
async fn update_token(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(token_ref): Path<String>,
    Json(payload): Json<UpdateTokenRequest>,
) -> impl IntoResponse {
    let record = match authorized_token(&state, &claims, &token_ref).await {
        Ok(record) => record,
        Err(response) => return response,
    };

    match TokenService::update_status(&state.db, &record.token, &payload.status).await {
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
    Path(token_ref): Path<String>,
) -> impl IntoResponse {
    let record = match authorized_token(&state, &claims, &token_ref).await {
        Ok(record) => record,
        Err(response) => return response,
    };

    match TokenService::delete(&state.db, &record.token).await {
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
    Path(token_ref): Path<String>,
    Json(payload): Json<RotateTokenRequest>,
) -> impl IntoResponse {
    let record = match authorized_token(&state, &claims, &token_ref).await {
        Ok(record) => record,
        Err(response) => return response,
    };

    match TokenService::rotate(
        &state.db,
        &record.token,
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
    Path(token_ref): Path<String>,
) -> impl IntoResponse {
    let record = match authorized_token(&state, &claims, &token_ref).await {
        Ok(record) => record,
        Err(response) => return response,
    };

    match TokenService::revoke_old_key(&state.db, &record.token).await {
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
    Path(token_ref): Path<String>,
    Json(payload): Json<SetIpWhitelistRequest>,
) -> impl IntoResponse {
    let record = match authorized_token(&state, &claims, &token_ref).await {
        Ok(record) => record,
        Err(response) => return response,
    };

    match TokenService::set_ip_whitelist(&state.db, &record.token, &payload.ip_whitelist).await {
        Ok(true) => ok(serde_json::json!({ "status": "updated" })).into_response(),
        Ok(false) => err_status(StatusCode::NOT_FOUND, "Token not found").into_response(),
        Err(e) => {
            tracing::error!("[API] set_ip_whitelist error: {}", e);
            err(e).into_response()
        }
    }
}

/// Execute a console smoke-test request through the same data-plane router used
/// by `/v1/*`, while keeping the selected bearer secret server-side.
#[tracing::instrument(skip_all)]
async fn playground_chat(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<PlaygroundChatRequest>,
) -> Response {
    let record = match authorized_token(&state, &claims, &payload.token_ref).await {
        Ok(record) => record,
        Err(response) => return response,
    };

    let body = serde_json::json!({
        "model": payload.model,
        "messages": payload.messages,
        "stream": false,
        "temperature": payload.temperature,
        "max_tokens": payload.max_tokens,
    });

    let request = match Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header(header::AUTHORIZATION, format!("Bearer {}", record.token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
    {
        Ok(request) => request,
        Err(error) => {
            tracing::error!(%error, "Failed to build console playground request");
            return err_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to build playground request",
            )
            .into_response();
        }
    };

    match state.data_plane.clone().oneshot(request).await {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(%error, "Console playground data-plane request failed");
            err_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Playground data-plane request failed",
            )
            .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::token_management_id;

    #[test]
    fn management_id_is_stable_and_not_the_bearer_secret() {
        let token = "bc_live_super-secret-value";
        let first = token_management_id(token);
        let second = token_management_id(token);
        assert_eq!(first, second);
        assert_ne!(first, token);
        assert!(first.starts_with("tok_"));
        assert_ne!(first, token_management_id("bc_live_other-secret-value"));
    }
}
