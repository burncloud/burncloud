use crate::api::response::{err, ok};
use crate::AppState;
use axum::{
    extract::{Json, Query, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use burncloud_service_user::UserServiceError;
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
pub struct TopupDto {
    pub user_id: String,
    /// Amount in nanodollars ($1 = 1_000_000_000)
    pub amount: i64,
    #[serde(default)]
    pub currency: Option<String>,
}

#[derive(Serialize)]
struct AuthData {
    id: String,
    username: String,
    roles: Vec<String>,
    token: String,
}

#[derive(Serialize)]
struct TopupData {
    balance: i64,
    currency: String,
}

#[derive(Serialize)]
struct UsernameAvailability {
    available: bool,
}

#[derive(Serialize)]
struct UserSummary {
    id: String,
    username: String,
    email: Option<String>,
    status: i32,
    balance_usd: i64,
    balance_cny: i64,
    preferred_currency: Option<String>,
    role: String,
    group: &'static str,
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

async fn topup(State(state): State<AppState>, Json(payload): Json<TopupDto>) -> impl IntoResponse {
    let currency = payload.currency.unwrap_or_else(|| "USD".to_string());
    match state
        .user_service
        .topup(&state.db, &payload.user_id, payload.amount, &currency)
        .await
    {
        Ok(balance) => ok(TopupData { balance, currency }).into_response(),
        Err(e) => err(e).into_response(),
    }
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterDto>,
) -> impl IntoResponse {
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
                Ok(auth_token) => ok(AuthData {
                    id: user_id,
                    username: payload.username,
                    roles,
                    token: auth_token.token,
                })
                .into_response(),
                Err(e) => {
                    eprintln!("Token generation error: {}", e);
                    err("Registration succeeded but token generation failed").into_response()
                }
            }
        }
        Err(UserServiceError::UserAlreadyExists) => err("用户名已存在").into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
pub struct CheckUsernameQuery {
    username: String,
}

async fn check_username(
    State(state): State<AppState>,
    Query(params): Query<CheckUsernameQuery>,
) -> impl IntoResponse {
    match state
        .user_service
        .is_username_available(&state.db, &params.username)
        .await
    {
        Ok(available) => ok(UsernameAvailability { available }).into_response(),
        Err(e) => err(e).into_response(),
    }
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginDto>,
) -> impl IntoResponse {
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
                roles,
                token: auth_token.token,
            })
            .into_response()
        }
        Err(UserServiceError::UserNotFound) => err("User not found").into_response(),
        Err(UserServiceError::InvalidCredentials) => err("Invalid credentials").into_response(),
        Err(e) => err(e).into_response(),
    }
}

async fn list_users(State(state): State<AppState>) -> impl IntoResponse {
    match state.user_service.list_users(&state.db).await {
        Ok(users) => {
            let mut summaries = Vec::new();
            for u in users {
                let roles = state
                    .user_service
                    .get_user_roles(&state.db, &u.id)
                    .await
                    .unwrap_or_default();
                let role = roles
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| "user".to_string());

                summaries.push(UserSummary {
                    id: u.id,
                    username: u.username,
                    email: u.email,
                    status: u.status,
                    balance_usd: u.balance_usd,
                    balance_cny: u.balance_cny,
                    preferred_currency: u.preferred_currency,
                    role,
                    group: "default",
                });
            }
            ok(summaries).into_response()
        }
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
pub struct RechargeQuery {
    user_id: Option<String>,
}

async fn list_recharges(
    State(state): State<AppState>,
    Query(params): Query<RechargeQuery>,
) -> impl IntoResponse {
    let user_id = params.user_id.as_deref().unwrap_or("demo-user");
    match state.user_service.list_recharges(&state.db, user_id).await {
        Ok(recharges) => ok(recharges).into_response(),
        Err(e) => err(e).into_response(),
    }
}
