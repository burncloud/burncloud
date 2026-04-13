use crate::AppState;
use axum::{
    extract::{Json, Query, State},
    response::Json as AxumJson,
    routing::{get, post},
    Router,
};
use burncloud_service_user::UserServiceError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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
    /// Amount in nanodollars (9 decimal precision: $1 = 1_000_000_000)
    pub amount: i64,
    /// Currency for the topup (USD or CNY, defaults to USD)
    #[serde(default)]
    pub currency: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/console/api/user/register", post(register))
        .route("/console/api/user/login", post(login))
        .route("/console/api/user/topup", post(topup))
        .route("/console/api/user/recharges", get(list_recharges))
        .route("/console/api/user/check_username", get(check_username))
        .route("/console/api/list_users", get(list_users))
}

async fn topup(State(state): State<AppState>, Json(payload): Json<TopupDto>) -> AxumJson<Value> {
    let currency = payload.currency.unwrap_or_else(|| "USD".to_string());
    match state
        .user_service
        .topup(&state.db, &payload.user_id, payload.amount, &currency)
        .await
    {
        Ok(balance) => AxumJson(
            json!({ "success": true, "data": { "balance": balance, "currency": currency } }),
        ),
        Err(e) => AxumJson(json!({ "success": false, "message": e.to_string() })),
    }
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterDto>,
) -> AxumJson<Value> {
    match state
        .user_service
        .register_user(&state.db, &payload.username, &payload.password, payload.email)
        .await
    {
        Ok(user_id) => {
            let roles = state
                .user_service
                .get_user_roles(&state.db, &user_id)
                .await
                .unwrap_or_default();

            match state.user_service.generate_token(&user_id, &payload.username) {
                Ok(auth_token) => AxumJson(json!({
                    "success": true,
                    "data": {
                        "id": user_id,
                        "username": payload.username,
                        "roles": roles,
                        "token": auth_token.token
                    }
                })),
                Err(e) => {
                    eprintln!("Token generation error: {}", e);
                    AxumJson(json!({ "success": false, "message": "Registration succeeded but token generation failed" }))
                }
            }
        }
        Err(UserServiceError::UserAlreadyExists) => {
            AxumJson(json!({ "success": false, "message": "用户名已存在" }))
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
    match state
        .user_service
        .is_username_available(&state.db, &params.username)
        .await
    {
        Ok(available) => AxumJson(json!({
            "success": true,
            "data": { "available": available }
        })),
        Err(e) => AxumJson(json!({ "success": false, "message": e.to_string() })),
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
        Err(e) => AxumJson(json!({ "success": false, "message": e.to_string() })),
    }
}

async fn list_users(State(state): State<AppState>) -> AxumJson<Value> {
    match state.user_service.list_users(&state.db).await {
        Ok(users) => {
            let mut safe_users = Vec::new();
            for u in users {
                let roles = state
                    .user_service
                    .get_user_roles(&state.db, &u.id)
                    .await
                    .unwrap_or_default();
                let role = roles.first().map(|s| s.as_str()).unwrap_or("user");

                safe_users.push(json!({
                    "id": u.id,
                    "username": u.username,
                    "email": u.email,
                    "status": u.status,
                    "balance_usd": u.balance_usd,
                    "balance_cny": u.balance_cny,
                    "preferred_currency": u.preferred_currency,
                    "role": role,
                    "group": "default"
                }));
            }
            AxumJson(json!({ "success": true, "data": safe_users }))
        }
        Err(e) => AxumJson(json!({ "success": false, "message": e.to_string() })),
    }
}

#[derive(Deserialize)]
pub struct RechargeQuery {
    user_id: Option<String>,
}

async fn list_recharges(
    State(state): State<AppState>,
    Query(params): Query<RechargeQuery>,
) -> AxumJson<Value> {
    let user_id = params.user_id.as_deref().unwrap_or("demo-user");
    match state
        .user_service
        .list_recharges(&state.db, user_id)
        .await
    {
        Ok(recharges) => AxumJson(json!({ "success": true, "data": recharges })),
        Err(e) => AxumJson(json!({ "success": false, "message": e.to_string() })),
    }
}
