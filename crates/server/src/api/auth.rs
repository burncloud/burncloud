use crate::api::response::{err, ok};
use crate::AppState;
use axum::{
    body::Body,
    extract::{Json, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::post,
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
    env::var("JWT_SECRET")
        .unwrap_or_else(|_| "burncloud-default-secret-change-in-production".to_string())
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

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/register", post(create_user))
        .route("/api/auth/login", post(login))
}

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
        Ok(user_id) => match state
            .user_service
            .generate_token(&user_id, &payload.username)
        {
            Ok(auth_token) => ok(AuthData {
                id: user_id,
                username: payload.username,
                roles: None,
                token: auth_token.token,
            })
            .into_response(),
            Err(e) => {
                eprintln!("JWT generation failed: {}", e);
                err("Failed to generate authentication token").into_response()
            }
        },
        Err(UserServiceError::UserAlreadyExists) => err("Username already exists").into_response(),
        Err(e) => {
            eprintln!("Registration error: {}", e);
            err("Registration failed").into_response()
        }
    }
}

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
            eprintln!("Login error: {}", e);
            err("Login failed").into_response()
        }
    }
}

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
