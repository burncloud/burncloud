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

#[derive(Deserialize, Serialize)]
pub struct CreateTokenRequest {
    pub user_id: String,
    pub quota_limit: Option<i64>,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateTokenRequest {
    pub status: String,
}

#[derive(Serialize)]
struct TokenOpResult {
    status: &'static str,
    token: String,
}

#[derive(Serialize)]
struct ApiError {
    error: String,
}

fn token_err(e: impl ToString) -> impl IntoResponse {
    Json(ApiError {
        error: e.to_string(),
    })
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/console/api/tokens", post(create_token).get(list_tokens))
        .route(
            "/console/api/tokens/{token}",
            get(delete_token).delete(delete_token).put(update_token),
        )
}

async fn list_tokens(State(state): State<AppState>) -> impl IntoResponse {
    tracing::info!("[API] list_tokens request received");
    match TokenService::list(&state.db).await {
        Ok(tokens) => {
            tracing::info!("[API] list_tokens success: found {} tokens", tokens.len());
            Json(tokens).into_response()
        }
        Err(e) => {
            tracing::error!("[API] list_tokens error: {}", e);
            token_err(e).into_response()
        }
    }
}

async fn create_token(
    State(state): State<AppState>,
    Json(payload): Json<CreateTokenRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "[API] create_token request: user_id={}, quota={:?}",
        payload.user_id,
        payload.quota_limit
    );

    let token_str = format!("sk-burncloud-{}", Uuid::new_v4());

    let db_token = RouterToken {
        token: token_str.clone(),
        user_id: payload.user_id,
        status: "active".to_string(),
        quota_limit: payload.quota_limit.unwrap_or(-1),
        used_quota: 0,
        accessed_time: 0,
        expired_time: -1,
    };

    match TokenService::create(&state.db, &db_token).await {
        Ok(_) => {
            tracing::info!("[API] create_token success: {}", token_str);
            Json(TokenOpResult {
                status: "created",
                token: token_str,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("[API] create_token error: {}", e);
            token_err(e).into_response()
        }
    }
}

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
            Json(TokenOpResult {
                status: "updated",
                token,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("[API] update_token error: {}", e);
            token_err(e).into_response()
        }
    }
}

async fn delete_token(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> impl IntoResponse {
    tracing::info!("[API] delete_token request: {}", token);
    match TokenService::delete(&state.db, &token).await {
        Ok(_) => {
            tracing::info!("[API] delete_token success");
            Json(TokenOpResult {
                status: "deleted",
                token,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("[API] delete_token error: {}", e);
            token_err(e).into_response()
        }
    }
}
