use axum::{
    extract::{State, Json, Path},
    response::Json as AxumJson,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::AppState;
use burncloud_database_user::{UserDatabase, DbUser};
use bcrypt::{hash, verify, DEFAULT_COST};
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

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/console/api/user/register", post(register))
        .route("/console/api/user/login", post(login))
        .route("/console/api/user", get(list_users))
        // .route("/console/api/user/me", get(get_current_user)) // Needs auth middleware context
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterDto>,
) -> AxumJson<Value> {
    // 1. Check if user exists
    match UserDatabase::get_user_by_username(&state.db, &payload.username).await {
        Ok(Some(_)) => return AxumJson(json!({ "success": false, "message": "Username already exists" })),
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
        username: payload.username,
        email: payload.email,
        password_hash: Some(password_hash),
        github_id: None,
    };

    match UserDatabase::create_user(&state.db, &user).await {
        Ok(_) => {
            // Assign default role
            let _ = UserDatabase::assign_role(&state.db, &user.id, "user").await;
            AxumJson(json!({ "success": true, "data": { "id": user.id, "username": user.username } }))
        },
        Err(e) => AxumJson(json!({ "success": false, "message": e.to_string() })),
    }
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginDto>,
) -> AxumJson<Value> {
    match UserDatabase::get_user_by_username(&state.db, &payload.username).await {
        Ok(Some(user)) => {
            if let Some(hash_str) = user.password_hash {
                if verify(&payload.password, &hash_str).unwrap_or(false) {
                    // Success
                    // In a real app, generate JWT here.
                    // For now, return user info.
                    let roles = UserDatabase::get_user_roles(&state.db, &user.id).await.unwrap_or_default();
                    return AxumJson(json!({ 
                        "success": true, 
                        "data": { 
                            "id": user.id, 
                            "username": user.username,
                            "roles": roles,
                            "token": "mock-token-for-now" 
                        } 
                    }));
                }
            }
            AxumJson(json!({ "success": false, "message": "Invalid credentials" }))
        },
        Ok(None) => AxumJson(json!({ "success": false, "message": "User not found" })),
        Err(e) => AxumJson(json!({ "success": false, "message": e.to_string() })),
    }
}

async fn list_users(
    State(state): State<AppState>,
) -> AxumJson<Value> {
    match UserDatabase::list_users(&state.db).await {
        Ok(users) => {
            // Filter out sensitive data if needed (e.g., password_hash)
            // But DbUser has #[serde(skip)]? No.
            // Let's map it manually to be safe.
            let safe_users: Vec<Value> = users.into_iter().map(|u| json!({
                "id": u.id,
                "username": u.username,
                "email": u.email,
                // "password_hash": u.password_hash // Don't return this
            })).collect();
            
            AxumJson(json!({ "success": true, "data": safe_users }))
        },
        Err(e) => AxumJson(json!({ "success": false, "message": e.to_string() })),
    }
}
