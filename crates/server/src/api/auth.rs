use crate::api::response::{err, ok};
use crate::AppState;
use axum::{
    body::Body,
    extract::{Json, State},
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use burncloud_service_user::UserServiceError;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

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
    burncloud_common::constants::jwt_secret()
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

/// Resolve whether the authenticated principal currently has the admin role.
/// Roles are read from the database instead of trusting client-provided claims.
pub async fn is_admin(state: &AppState, claims: &Claims) -> Result<bool, StatusCode> {
    state
        .user_service
        .get_user_roles(&state.db, &claims.sub)
        .await
        .map(|roles| roles.iter().any(|role| role == "admin"))
        .map_err(|e| {
            tracing::error!(user_id = %claims.sub, error = %e, "Failed to resolve admin role");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Authorization middleware for administrator-only Console routes.
/// Authentication must run outside this layer so `Claims` are already present.
#[tracing::instrument(skip_all)]
pub async fn admin_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let claims = req
        .extensions()
        .get::<Claims>()
        .cloned()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if is_admin(&state, &claims).await? {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

fn is_data_plane_path(path: &str) -> bool {
    path == "/v1"
        || path.starts_with("/v1/")
        || path == "/api/v1"
        || path.starts_with("/api/v1/")
}

fn is_sensitive_internal_mutation(method: &Method, path: &str) -> bool {
    method == Method::POST
        && matches!(
            path,
            "/console/internal/prices/sync" | "/console/internal/circuit-breaker/trip-all"
        )
}

/// Enforce the trust boundary between the management plane, data plane, and
/// sensitive internal control-plane mutations.
///
/// Invariants:
/// - a Console JWT is never accepted as an inference credential;
/// - sensitive internal POST endpoints fail closed unless
///   `BURNCLOUD_INTERNAL_SECRET` is configured and presented.
#[tracing::instrument(skip_all)]
pub async fn security_boundary_middleware(
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();

    if is_sensitive_internal_mutation(req.method(), path) {
        let expected = std::env::var("BURNCLOUD_INTERNAL_SECRET")
            .ok()
            .filter(|secret| !secret.is_empty())
            .ok_or_else(|| {
                tracing::error!(
                    path,
                    "Sensitive internal endpoint disabled: BURNCLOUD_INTERNAL_SECRET is not configured"
                );
                StatusCode::SERVICE_UNAVAILABLE
            })?;
        let provided = req
            .headers()
            .get("x-internal-secret")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");
        if provided != expected {
            tracing::warn!(path, "Rejected unauthorized internal mutation");
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    if is_data_plane_path(path) {
        let credential = req
            .headers()
            .get("authorization")
            .and_then(|header| header.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .or_else(|| {
                req.headers()
                    .get("x-api-key")
                    .and_then(|header| header.to_str().ok())
            })
            .or_else(|| {
                req.headers()
                    .get("x-goog-api-key")
                    .and_then(|header| header.to_str().ok())
            });

        if credential.is_some_and(|token| verify_jwt(token).is_ok()) {
            tracing::warn!(path, "Rejected Console JWT on data-plane route");
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    Ok(next.run(req).await)
}

/// Public routes - no authentication required
/// - /api/auth/register - Registration
/// - /api/auth/login - Login
/// - /api/auth/forgot-password - Forgot password
/// - /api/auth/reset-password - Reset password
/// - /api/auth/google - Google OAuth
/// - /api/auth/github - GitHub OAuth
pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/register", post(create_user))
        .route("/api/auth/login", post(login))
        .route("/api/auth/forgot-password", post(forgot_password))
        .route("/api/auth/reset-password", post(reset_password))
        .route("/api/auth/google", get(oauth_google))
        .route("/api/auth/github", get(oauth_github))
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

#[cfg(test)]
mod tests {
    use super::verify_jwt;
    use burncloud_service_user::UserService;

    #[test]
    fn verify_jwt_accepts_tokens_signed_by_user_service() {
        let service = UserService::new();
        let auth = service
            .generate_token("user-1", "alice")
            .expect("token generation");

        let claims = verify_jwt(&auth.token).expect("middleware must accept UserService JWT");
        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.username, "alice");
    }
}
