use crate::AppState;
use axum::{
    extract::{Json, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json as AxumJson, Response},
    routing::post,
    Router,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use burncloud_database_user::{DbUser, UserDatabase};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use uuid::Uuid;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,      // Subject (user ID)
    pub username: String, // Username
    pub exp: usize,       // Expiration time (as UTC timestamp)
    pub iat: usize,       // Issued at (as UTC timestamp)
}

fn get_jwt_secret() -> String {
    env::var("JWT_SECRET").unwrap_or_else(|_| "burncloud-default-secret-change-in-production".to_string())
}

fn generate_jwt(user_id: &str, username: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp: now + 86400 * 7, // 7 days
        iat: now,
    };
    
    let secret = get_jwt_secret();
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
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
    // 1. Check if user exists
    match UserDatabase::get_user_by_username(&state.db, &payload.username).await {
        Ok(Some(_)) => {
            return AxumJson(json!({ "success": false, "message": "Username already exists" }))
        }
        Err(e) => return AxumJson(json!({ "success": false, "message": e.to_string() })),
        _ => {}
    }

    // 2. Hash password
    let password_hash = match hash(&payload.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => return AxumJson(json!({ "success": false, "message": e.to_string() })),
    };

    // 3. Create user
    let user = DbUser {
        id: Uuid::new_v4().to_string(),
        username: payload.username.clone(),
        email: payload.email,
        password_hash: Some(password_hash),
        github_id: None,
        status: 1,
        balance: 10.0, // Signup bonus
    };

    match UserDatabase::create_user(&state.db, &user).await {
        Ok(_) => {
            // Assign default role
            let _ = UserDatabase::assign_role(&state.db, &user.id, "user").await;
            
            // Generate JWT
            match generate_jwt(&user.id, &user.username) {
                Ok(token) => AxumJson(json!({
                    "success": true,
                    "data": {
                        "id": user.id,
                        "username": user.username,
                        "token": token
                    }
                })),
                Err(e) => {
                    eprintln!("JWT generation failed: {}", e);
                    AxumJson(json!({ "success": false, "message": "Failed to generate authentication token" }))
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to create user: {}", e);
            AxumJson(json!({ "success": false, "message": "Failed to create user account" }))
        }
    }
}

async fn login(State(state): State<AppState>, Json(payload): Json<LoginDto>) -> AxumJson<Value> {
    match UserDatabase::get_user_by_username(&state.db, &payload.username).await {
        Ok(Some(user)) => {
            if let Some(hash_str) = user.password_hash {
                match verify(&payload.password, &hash_str) {
                    Ok(true) => {
                        // Success - generate JWT
                        let roles = UserDatabase::get_user_roles(&state.db, &user.id)
                            .await
                            .unwrap_or_default();
                        
                        match generate_jwt(&user.id, &user.username) {
                            Ok(token) => {
                                return AxumJson(json!({
                                    "success": true,
                                    "data": {
                                        "id": user.id,
                                        "username": user.username,
                                        "roles": roles,
                                        "token": token
                                    }
                                }));
                            }
                            Err(e) => {
                                eprintln!("JWT generation failed: {}", e);
                                return AxumJson(json!({ "success": false, "message": "Failed to generate authentication token" }));
                            }
                        }
                    }
                    Ok(false) => {
                        // Invalid password
                    }
                    Err(e) => {
                        eprintln!("Password verification error: {}", e);
                        // Treat as invalid password for security
                    }
                }
            }
            AxumJson(json!({ "success": false, "message": "Invalid credentials" }))
        }
        Ok(None) => AxumJson(json!({ "success": false, "message": "User not found" })),
        Err(e) => {
            eprintln!("Database error during login: {}", e);
            AxumJson(json!({ "success": false, "message": "Login failed" }))
        }
    }
}

// Auth Middleware
pub async fn auth_middleware<B>(
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
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
            // Add claims to request extensions for use in handlers
            req.extensions_mut().insert(claims);
            Ok(next.run(req).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}
