use crate::AppState;
use axum::{
    body::Body,
    extract::{Json, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::{Json as AxumJson, Response},
    routing::post,
    Router,
};
use burncloud_service_user::UserServiceError;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
    pub sub: String,      // Subject (user ID)
    pub username: String, // Username
    pub exp: usize,       // Expiration time (as UTC timestamp)
    pub iat: usize,       // Issued at (as UTC timestamp)
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
) -> AxumJson<Value> {
    match state
        .user_service
        .register_user(&state.db, &payload.username, &payload.password, payload.email)
        .await
    {
        Ok(user_id) => {
            match state.user_service.generate_token(&user_id, &payload.username) {
                Ok(auth_token) => AxumJson(json!({
                    "success": true,
                    "data": {
                        "id": user_id,
                        "username": payload.username,
                        "token": auth_token.token
                    }
                })),
                Err(e) => {
                    eprintln!("JWT generation failed: {}", e);
                    AxumJson(
                        json!({ "success": false, "message": "Failed to generate authentication token" }),
                    )
                }
            }
        }
        Err(UserServiceError::UserAlreadyExists) => {
            AxumJson(json!({ "success": false, "message": "Username already exists" }))
        }
        Err(e) => {
            eprintln!("Registration error: {}", e);
            AxumJson(json!({ "success": false, "message": "Registration failed" }))
        }
    }
}

async fn login(State(state): State<AppState>, Json(payload): Json<LoginDto>) -> AxumJson<Value> {
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

            AxumJson(json!({
                "success": true,
                "data": {
                    "id": auth_token.user_id,
                    "username": auth_token.username,
                    "roles": roles,
                    "token": auth_token.token
                }
            }))
        }
        Err(UserServiceError::UserNotFound) => {
            AxumJson(json!({ "success": false, "message": "User not found" }))
        }
        Err(UserServiceError::InvalidCredentials) => {
            AxumJson(json!({ "success": false, "message": "Invalid credentials" }))
        }
        Err(e) => {
            eprintln!("Login error: {}", e);
            AxumJson(json!({ "success": false, "message": "Login failed" }))
        }
    }
}

// Auth Middleware
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
