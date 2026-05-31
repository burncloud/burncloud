use crate::api::response::{err, ok};
use crate::AppState;
use axum::{
    body::Body,
    extract::{Json, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use burncloud_service_user::UserServiceError;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Deserialize)]
pub struct RegisterDto {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginDto {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct ForgotPasswordDto {
    pub email: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordDto {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Serialize)]
struct AuthData {
    id: String,
    username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    roles: Option<Vec<String>>,
    token: String,
}

fn get_jwt_secret() -> String {
    env::var("JWT_SECRET").unwrap_or_else(|_| "default-secret-key-change-in-production".to_string())
}

pub fn verify_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secret = get_jwt_secret();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

/// Public routes - no authentication required
/// - /api/auth/register - Registration
/// - /api/auth/login - Login
/// - /console/api/auth/forgot-password - Forgot password
/// - /console/api/auth/reset-password - Reset password
/// - /console/api/auth/google - Google OAuth
/// - /console/api/auth/github - GitHub OAuth
pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/register", post(create_user))
        .route("/api/auth/login", post(login))
        .route("/console/api/auth/forgot-password", post(forgot_password))
        .route("/console/api/auth/reset-password", post(reset_password))
        .route("/console/api/auth/google", get(oauth_google))
        .route("/console/api/auth/github", get(oauth_github))
}

/// Protected routes - authentication required
/// Currently empty, but available for future protected auth endpoints
/// (e.g., logout, change-password)
pub fn protected_routes() -> Router<AppState> {
    Router::new()
    // Add protected auth routes here when needed:
    // .route("/console/api/auth/logout", post(logout))
    // .route("/console/api/auth/change-password", post(change_password))
}

#[tracing::instrument(skip(state, payload), fields(username = %payload.username))]
async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<RegisterDto>,
) -> impl IntoResponse {
    match state
        .user_service
        .register_user(
            &state.db,
            &payload.username,
            &payload.password,
            payload.email,
        )
        .await
    {
        Ok(user_id) => {
            let roles = state
                .user_service
                .get_user_roles(&state.db, &user_id)
                .await
                .unwrap_or_default();
            match state
                .user_service
                .generate_token(&user_id, &payload.username)
            {
                Ok(auth_token) => ok(AuthData {
                    id: user_id,
                    username: payload.username,
                    roles: Some(roles),
                    token: auth_token.token,
                })
                .into_response(),
                Err(e) => {
                    tracing::error!("JWT generation failed: {}", e);
                    err("Failed to generate authentication token").into_response()
                }
            }
        }
        Err(UserServiceError::UserAlreadyExists) => err("Username already exists").into_response(),
        Err(e) => {
            tracing::error!("Registration error: {}", e);
            err("Registration failed").into_response()
        }
    }
}

#[tracing::instrument(skip(state, payload), fields(username = %payload.username))]
async fn login(State(state): State<AppState>, Json(payload): Json<LoginDto>) -> impl IntoResponse {
    match state
        .user_service
        .login_user(&state.db, &payload.username, &payload.password)
        .await
    {
        Ok(auth_token) => {
            let roles = state
                .user_service
                .get_user_roles(&state.db, &auth_token.user_id)
                .await
                .unwrap_or_default();

            ok(AuthData {
                id: auth_token.user_id,
                username: auth_token.username,
                roles: Some(roles),
                token: auth_token.token,
            })
            .into_response()
        }
        Err(UserServiceError::UserNotFound) => err("User not found").into_response(),
        Err(UserServiceError::InvalidCredentials) => err("Invalid credentials").into_response(),
        Err(e) => {
            tracing::error!("Login error: {}", e);
            err("Login failed").into_response()
        }
    }
}

async fn forgot_password(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordDto>,
) -> impl IntoResponse {
    match state
        .user_service
        .request_password_reset(&state.db, &payload.email)
        .await
    {
        Ok(_reset_token) => {
            tracing::info!("Password reset token generated for {}", payload.email);
            ok(serde_json::json!({ "message": "If the email exists, a reset token has been generated" })).into_response()
        }
        Err(UserServiceError::UserNotFound) => {
            // Return success even if user not found to prevent email enumeration
            ok(serde_json::json!({ "message": "If the email exists, a reset token has been generated" })).into_response()
        }
        Err(e) => {
            tracing::error!("Forgot password error: {}", e);
            err("Failed to process password reset request").into_response()
        }
    }
}

async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordDto>,
) -> impl IntoResponse {
    match state
        .user_service
        .reset_password(&state.db, &payload.token, &payload.new_password)
        .await
    {
        Ok(()) => ok(serde_json::json!({ "message": "Password reset successful" })).into_response(),
        Err(UserServiceError::InvalidCredentials) => {
            err("Invalid or expired reset token").into_response()
        }
        Err(e) => {
            tracing::error!("Reset password error: {}", e);
            err("Password reset failed").into_response()
        }
    }
}

async fn oauth_google(State(_state): State<AppState>) -> impl IntoResponse {
    match burncloud_service_user::UserService::oauth_url("google") {
        Ok(url) => ok(serde_json::json!({ "url": url })).into_response(),
        Err(e) => {
            tracing::error!("Google OAuth URL error: {}", e);
            err("Failed to generate Google OAuth URL").into_response()
        }
    }
}

async fn oauth_github(State(_state): State<AppState>) -> impl IntoResponse {
    match burncloud_service_user::UserService::oauth_url("github") {
        Ok(url) => ok(serde_json::json!({ "url": url })).into_response(),
        Err(e) => {
            tracing::error!("GitHub OAuth URL error: {}", e);
            err("Failed to generate GitHub OAuth URL").into_response()
        }
    }
}

/// Authentication middleware for protected routes.
/// Validates JWT token from Authorization header and injects Claims into request extensions.
#[tracing::instrument(skip_all)]
pub async fn auth_middleware(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let token = if let Some(auth_header) = auth_header {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            token
        } else {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    match verify_jwt(token) {
        Ok(claims) => {
            req.extensions_mut().insert(claims);
            Ok(next.run(req).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}
