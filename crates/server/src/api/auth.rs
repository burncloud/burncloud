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
use bcrypt::{hash, verify, DEFAULT_COST};
use burncloud_database_user::{DbUser, UserDatabase};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use uuid::Uuid;

/// Minimum password length
const MIN_PASSWORD_LENGTH: usize = 6;

/// Minimum username length
const MIN_USERNAME_LENGTH: usize = 3;

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
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
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

/// Validate username format and length
fn validate_username(username: &str) -> Result<(), String> {
    let trimmed = username.trim();

    if trimmed.is_empty() {
        return Err("Username cannot be empty".to_string());
    }

    if trimmed.len() < MIN_USERNAME_LENGTH {
        return Err(format!(
            "Username must be at least {} characters long",
            MIN_USERNAME_LENGTH
        ));
    }

    if !trimmed
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(
            "Username can only contain letters, numbers, underscores, and hyphens".to_string(),
        );
    }

    Ok(())
}

/// Validate password strength
fn validate_password(password: &str) -> Result<(), String> {
    if password.is_empty() {
        return Err("Password cannot be empty".to_string());
    }

    if password.len() < MIN_PASSWORD_LENGTH {
        return Err(format!(
            "Password must be at least {} characters long",
            MIN_PASSWORD_LENGTH
        ));
    }

    let has_letter = password.chars().any(|c| c.is_alphabetic());
    let has_number = password.chars().any(|c| c.is_numeric());

    if !has_letter || !has_number {
        return Err("Password must contain at least one letter and one number".to_string());
    }

    Ok(())
}

/// Validate email format
fn validate_email(email: &str) -> Result<(), String> {
    if email.is_empty() {
        return Ok(()); // Empty email is allowed (optional field)
    }

    // Check for basic email format: something@something.something
    if !email.contains('@') {
        return Err("Invalid email format".to_string());
    }

    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err("Invalid email format".to_string());
    }

    let (local, domain) = (parts[0], parts[1]);

    // Local part should not be empty
    if local.is_empty() {
        return Err("Invalid email format".to_string());
    }

    // Domain should have at least a dot and something after it
    if !domain.contains('.') || domain.ends_with('.') || domain.starts_with('.') {
        return Err("Invalid email format".to_string());
    }

    Ok(())
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
    // Validate username
    if let Err(msg) = validate_username(&payload.username) {
        return AxumJson(json!({ "success": false, "message": msg }));
    }

    // Validate password
    if let Err(msg) = validate_password(&payload.password) {
        return AxumJson(json!({ "success": false, "message": msg }));
    }

    // Validate email if provided
    if let Some(ref email) = payload.email {
        if let Err(msg) = validate_email(email) {
            return AxumJson(json!({ "success": false, "message": msg }));
        }
    }

    // Check if user exists
    match UserDatabase::get_user_by_username(&state.db, &payload.username).await {
        Ok(Some(_)) => {
            return AxumJson(json!({ "success": false, "message": "Username already exists" }))
        }
        Err(e) => {
            eprintln!("Database error checking username: {}", e);
            return AxumJson(json!({ "success": false, "message": "Registration failed" }));
        }
        _ => {}
    }

    // Hash password
    let password_hash = match hash(&payload.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Password hashing error: {}", e);
            return AxumJson(json!({ "success": false, "message": "Registration failed" }));
        }
    };

    // Create user
    let user = DbUser {
        id: Uuid::new_v4().to_string(),
        username: payload.username.clone(),
        email: payload.email,
        password_hash: Some(password_hash),
        github_id: None,
        status: 1,
        balance_usd: 10_000_000_000, // 10 USD signup bonus in nanodollars
        balance_cny: 0,
        preferred_currency: Some("USD".to_string()),
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
                    AxumJson(
                        json!({ "success": false, "message": "Failed to generate authentication token" }),
                    )
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
                                return AxumJson(
                                    json!({ "success": false, "message": "Failed to generate authentication token" }),
                                );
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
            // Add claims to request extensions for use in handlers
            req.extensions_mut().insert(claims);
            Ok(next.run(req).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_username() {
        // Valid usernames
        assert!(validate_username("abc").is_ok());
        assert!(validate_username("test_user").is_ok());
        assert!(validate_username("test-user").is_ok());
        assert!(validate_username("user123").is_ok());

        // Invalid usernames
        assert!(validate_username("").is_err());
        assert!(validate_username("ab").is_err());
        assert!(validate_username("   ").is_err());
        assert!(validate_username("test@user").is_err());
        assert!(validate_username("test user").is_err());
    }

    #[test]
    fn test_validate_password() {
        // Valid passwords
        assert!(validate_password("abc123").is_ok());
        assert!(validate_password("password1").is_ok());
        assert!(validate_password("SecurePass123!").is_ok());

        // Invalid passwords
        assert!(validate_password("").is_err());
        assert!(validate_password("12345").is_err());
        assert!(validate_password("abcde").is_err());
        assert!(validate_password("abcdef").is_err());
        assert!(validate_password("123456").is_err());
    }

    #[test]
    fn test_validate_email() {
        // Valid emails
        assert!(validate_email("").is_ok());
        assert!(validate_email("test@example.com").is_ok());

        // Invalid emails
        assert!(validate_email("invalid-email").is_err());
        assert!(validate_email("test@").is_err());
    }
}
