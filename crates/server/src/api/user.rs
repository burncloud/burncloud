use crate::AppState;
use axum::{
    extract::{Json, Query, State},
    response::Json as AxumJson,
    routing::{get, post},
    Router,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use burncloud_database_user::{DbUser, UserDatabase};
use serde::Deserialize;
use serde_json::{json, Value};
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

#[derive(Deserialize)]
pub struct TopupDto {
    pub user_id: String,
    pub amount: f64,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/console/api/user/register", post(register))
        .route("/console/api/user/login", post(login))
        .route("/console/api/user/topup", post(topup))
        .route("/console/api/user/check_username", get(check_username))
        .route("/console/api/list_users", get(list_users))
    // .route("/console/api/user/me", get(get_current_user)) // Needs auth middleware context
}

async fn topup(State(state): State<AppState>, Json(payload): Json<TopupDto>) -> AxumJson<Value> {
    match UserDatabase::update_balance(&state.db, &payload.user_id, payload.amount).await {
        Ok(new_balance) => AxumJson(json!({ "success": true, "data": { "balance": new_balance } })),
        Err(e) => AxumJson(json!({ "success": false, "message": e.to_string() })),
    }
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterDto>,
) -> AxumJson<Value> {
    // 1. Check if user exists
    match UserDatabase::get_user_by_username(&state.db, &payload.username).await {
        Ok(Some(_)) => return AxumJson(json!({ "success": false, "message": "用户名已存在" })),
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

            // Return login data for auto-login
            // NOTE: This is a behavior change - register now returns login data
            // This enables auto-login feature without requiring separate login call
            let roles = UserDatabase::get_user_roles(&state.db, &user.id)
                .await
                .unwrap_or_default();

            // TODO: Generate proper JWT token instead of mock token
            // For production, implement JWT signing with secret key and expiration
            AxumJson(json!({
                "success": true,
                "data": {
                    "id": user.id,
                    "username": user.username,
                    "roles": roles,
                    "token": "mock-token-for-now"  // SECURITY: Replace with real JWT
                }
            }))
        }
        Err(e) => AxumJson(json!({ "success": false, "message": e.to_string() })),
    }
}

#[derive(Deserialize)]
pub struct CheckUsernameQuery {
    username: String,
}

async fn check_username(
    State(state): State<AppState>,
    Query(params): Query<CheckUsernameQuery>,
) -> AxumJson<Value> {
    match UserDatabase::get_user_by_username(&state.db, &params.username).await {
        Ok(Some(_)) => {
            // Username exists
            AxumJson(json!({
                "success": true,
                "data": { "available": false }
            }))
        }
        Ok(None) => {
            // Username available
            AxumJson(json!({
                "success": true,
                "data": { "available": true }
            }))
        }
        Err(e) => AxumJson(json!({
            "success": false,
            "message": e.to_string()
        })),
    }
}

async fn login(State(state): State<AppState>, Json(payload): Json<LoginDto>) -> AxumJson<Value> {
    match UserDatabase::get_user_by_username(&state.db, &payload.username).await {
        Ok(Some(user)) => {
            if let Some(hash_str) = user.password_hash {
                if verify(&payload.password, &hash_str).unwrap_or(false) {
                    // Success
                    // In a real app, generate JWT here.
                    // For now, return user info.
                    let roles = UserDatabase::get_user_roles(&state.db, &user.id)
                        .await
                        .unwrap_or_default();
                    // TODO: Generate proper JWT token instead of mock token
                    // For production, implement JWT signing with secret key and expiration
                    return AxumJson(json!({
                        "success": true,
                        "data": {
                            "id": user.id,
                            "username": user.username,
                            "roles": roles,
                            "token": "mock-token-for-now"  // SECURITY: Replace with real JWT
                        }
                    }));
                }
            }
            AxumJson(json!({ "success": false, "message": "Invalid credentials" }))
        }
        Ok(None) => AxumJson(json!({ "success": false, "message": "User not found" })),
        Err(e) => AxumJson(json!({ "success": false, "message": e.to_string() })),
    }
}

async fn list_users(State(state): State<AppState>) -> AxumJson<Value> {
    match UserDatabase::list_users(&state.db).await {
        Ok(users) => {
            let mut safe_users = Vec::new();
            for u in users {
                // Fetch role (N+1 query for MVP)
                let roles = UserDatabase::get_user_roles(&state.db, &u.id)
                    .await
                    .unwrap_or_default();
                let role = roles.first().map(|s| s.as_str()).unwrap_or("user");

                safe_users.push(json!({
                    "id": u.id,
                    "username": u.username,
                    "email": u.email,
                    "status": u.status,
                    "balance": u.balance,
                    "role": role,
                    "group": "default" // Placeholder for group
                }));
            }

            AxumJson(json!({ "success": true, "data": safe_users }))
        }
        Err(e) => AxumJson(json!({ "success": false, "message": e.to_string() })),
    }
}
