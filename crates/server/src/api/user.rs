use crate::AppState;
use axum::{
    extract::{Json, Query, State},
    response::Json as AxumJson,
    routing::{get, post},
    Router,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use burncloud_database_user::{DbRecharge, DbUser, UserDatabase};
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

    let recharge = DbRecharge {
        id: 0,
        user_id: payload.user_id.clone(),
        amount: payload.amount,
        currency: Some(currency.clone()),
        description: Some("账户充值".to_string()),
        created_at: None,
    };
    match UserDatabase::create_recharge(&state.db, &recharge).await {
        Ok(_) => match UserDatabase::get_user_by_username(&state.db, "demo-user").await {
            Ok(Some(u)) => {
                // Return balance in nanodollars
                let balance = if currency == "CNY" {
                    u.balance_cny
                } else {
                    u.balance_usd
                };
                AxumJson(
                    json!({ "success": true, "data": { "balance": balance, "currency": currency } }),
                )
            }
            _ => AxumJson(json!({ "success": true })),
        },
        Err(e) => AxumJson(json!({ "success": false, "message": e.to_string() })),
    }
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterDto>,
) -> AxumJson<Value> {
    match UserDatabase::get_user_by_username(&state.db, &payload.username).await {
        Ok(Some(_)) => return AxumJson(json!({ "success": false, "message": "用户名已存在" })),
        Err(e) => return AxumJson(json!({ "success": false, "message": e.to_string() })),
        _ => {}
    }

    let password_hash = match hash(&payload.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => return AxumJson(json!({ "success": false, "message": e.to_string() })),
    };

    let user = DbUser {
        id: Uuid::new_v4().to_string(),
        username: payload.username.clone(),
        email: payload.email,
        password_hash: Some(password_hash),
        github_id: None,
        status: 1,
        balance_usd: 10_000_000_000, // 10 USD in nanodollars
        balance_cny: 0,
        preferred_currency: Some("USD".to_string()),
    };

    match UserDatabase::create_user(&state.db, &user).await {
        Ok(_) => {
            let _ = UserDatabase::assign_role(&state.db, &user.id, "user").await;
            let roles = UserDatabase::get_user_roles(&state.db, &user.id)
                .await
                .unwrap_or_default();

            AxumJson(json!({
                "success": true,
                "data": {
                    "id": user.id,
                    "username": user.username,
                    "roles": roles,
                    "token": "mock-token-for-now"
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
        Ok(Some(_)) => AxumJson(json!({
            "success": true,
            "data": { "available": false }
        })),
        Ok(None) => AxumJson(json!({
            "success": true,
            "data": { "available": true }
        })),
        Err(e) => AxumJson(json!({ "success": false, "message": e.to_string() })),
    }
}

async fn login(State(state): State<AppState>, Json(payload): Json<LoginDto>) -> AxumJson<Value> {
    match UserDatabase::get_user_by_username(&state.db, &payload.username).await {
        Ok(Some(user)) => {
            if let Some(hash_str) = user.password_hash {
                if verify(&payload.password, &hash_str).unwrap_or(false) {
                    let roles = UserDatabase::get_user_roles(&state.db, &user.id)
                        .await
                        .unwrap_or_default();
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
                let roles = UserDatabase::get_user_roles(&state.db, &u.id)
                    .await
                    .unwrap_or_default();
                let role = roles.first().map(|s| s.as_str()).unwrap_or("user");

                // Balances are returned in nanodollars (i64)
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

async fn list_recharges(
    State(state): State<AppState>,
    Query(params): Query<Value>,
) -> AxumJson<Value> {
    let user_id = params
        .get("user_id")
        .and_then(|v| v.as_str())
        .unwrap_or("demo-user");
    match UserDatabase::list_recharges(&state.db, user_id).await {
        Ok(recharges) => AxumJson(json!({ "success": true, "data": recharges })),
        Err(e) => AxumJson(json!({ "success": false, "message": e.to_string() })),
    }
}
