#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types)]

mod test_utils;

use burncloud_database::Database;
use burncloud_database_router::RouterToken;
use burncloud_service_token::TokenService;
use burncloud_service_user::UserService;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::sync::Arc;

const JWT_SECRET: &str = "burncloud-security-invariant-jwt-secret-2026";
const INTERNAL_SECRET: &str = "burncloud-security-invariant-internal-secret";

fn configure_security_env() {
    std::env::set_var("JWT_SECRET", JWT_SECRET);
    std::env::set_var("BURNCLOUD_INTERNAL_SECRET", INTERNAL_SECRET);
    std::env::set_var("SKIP_INITIAL_PRICE_SYNC", "1");
}

async fn spawn_server(db: Arc<Database>) -> anyhow::Result<String> {
    let app = burncloud_server::create_app(db, false).await?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .unwrap_or_else(|e| panic!("security invariant server failed: {e}"));
    });
    Ok(format!("http://{addr}"))
}

async fn create_principals(db: &Database) -> anyhow::Result<(String, String, String, String)> {
    let service = UserService::new();
    let admin_id = service
        .register_user(db, "invariant-admin", "test-password", None)
        .await?;
    let user_id = service
        .register_user(db, "invariant-user", "test-password", None)
        .await?;

    let admin_jwt = service.generate_token(&admin_id, "invariant-admin")?.token;
    let user_jwt = service.generate_token(&user_id, "invariant-user")?.token;

    Ok((admin_id, admin_jwt, user_id, user_jwt))
}

fn router_token(token: &str, user_id: &str) -> RouterToken {
    RouterToken {
        token: token.to_string(),
        user_id: user_id.to_string(),
        status: "active".to_string(),
        quota_limit: -1,
        used_quota: 0,
        expired_time: -1,
        accessed_time: 0,
        key_version: 1,
        old_key_hash: None,
        old_key_expires_at: 0,
        ip_whitelist: None,
        key_prefix: "bc_live_".to_string(),
        created_at: 0,
        last_rotated_at: 0,
    }
}

#[tokio::test]
async fn console_jwt_cannot_authenticate_data_plane() -> anyhow::Result<()> {
    configure_security_env();
    let db = test_utils::make_isolated_db().await;
    let (_admin_id, _admin_jwt, user_id, user_jwt) = create_principals(&db).await?;
    let api_key = "bc_live_security_data_plane_key";
    TokenService::create(&db, &router_token(api_key, &user_id)).await?;
    let base = spawn_server(db).await?;
    let client = Client::new();
    let body = serde_json::json!({
        "model": "security-invariant-model",
        "messages": [{"role": "user", "content": "hello"}]
    });

    let jwt_response = client
        .post(format!("{base}/v1/chat/completions"))
        .bearer_auth(&user_jwt)
        .json(&body)
        .send()
        .await?;
    assert_eq!(
        jwt_response.status(),
        StatusCode::UNAUTHORIZED,
        "management JWTs must never be accepted as inference credentials"
    );

    let api_key_response = client
        .post(format!("{base}/v1/chat/completions"))
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;
    assert_ne!(
        api_key_response.status(),
        StatusCode::UNAUTHORIZED,
        "the management/data-plane boundary must not reject a real API credential"
    );

    Ok(())
}

#[tokio::test]
async fn regular_users_cannot_execute_admin_management_actions() -> anyhow::Result<()> {
    configure_security_env();
    let db = test_utils::make_isolated_db().await;
    let (_admin_id, admin_jwt, user_id, user_jwt) = create_principals(&db).await?;
    let base = spawn_server(db).await?;
    let client = Client::new();

    let logs = client
        .get(format!("{base}/console/api/logs"))
        .bearer_auth(&user_jwt)
        .send()
        .await?;
    assert_eq!(logs.status(), StatusCode::FORBIDDEN);

    let topup = client
        .post(format!("{base}/console/api/user/topup"))
        .bearer_auth(&user_jwt)
        .json(&serde_json::json!({
            "user_id": user_id,
            "amount": 1_000_000_000_i64,
            "currency": "USD"
        }))
        .send()
        .await?;
    assert_eq!(
        topup.status(),
        StatusCode::FORBIDDEN,
        "a normal account must not be able to mint balance"
    );

    let admin_logs = client
        .get(format!("{base}/console/api/logs"))
        .bearer_auth(&admin_jwt)
        .send()
        .await?;
    assert_eq!(
        admin_logs.status(),
        StatusCode::OK,
        "admin access must remain functional"
    );

    Ok(())
}

#[tokio::test]
async fn token_management_is_owner_scoped_and_redacted() -> anyhow::Result<()> {
    configure_security_env();
    let db = test_utils::make_isolated_db().await;
    let (admin_id, admin_jwt, user_id, user_jwt) = create_principals(&db).await?;
    let admin_key = "bc_live_admin_secret_1234";
    let user_key = "bc_live_user_secret_5678";
    TokenService::create(&db, &router_token(admin_key, &admin_id)).await?;
    TokenService::create(&db, &router_token(user_key, &user_id)).await?;
    let base = spawn_server(db.clone()).await?;
    let client = Client::new();

    let user_list = client
        .get(format!("{base}/console/api/tokens"))
        .bearer_auth(&user_jwt)
        .send()
        .await?;
    assert_eq!(user_list.status(), StatusCode::OK);
    let user_body = user_list.text().await?;
    assert!(
        !user_body.contains(user_key),
        "token lists must redact bearer secrets"
    );
    assert!(
        !user_body.contains(admin_key),
        "users must not see another owner's secret"
    );
    assert!(
        user_body.contains("5678"),
        "owner should receive a non-secret token hint"
    );
    assert!(
        !user_body.contains("1234"),
        "owner list must exclude other users' tokens"
    );

    let forbidden_delete = client
        .delete(format!("{base}/console/api/tokens/{admin_key}"))
        .bearer_auth(&user_jwt)
        .send()
        .await?;
    assert_eq!(forbidden_delete.status(), StatusCode::FORBIDDEN);
    assert!(TokenService::validate(&db, admin_key).await?.is_some());

    let owner_get = client
        .get(format!("{base}/console/api/tokens/{user_key}"))
        .bearer_auth(&user_jwt)
        .send()
        .await?;
    assert_eq!(owner_get.status(), StatusCode::OK);
    let owner_body = owner_get.text().await?;
    assert!(!owner_body.contains(user_key));
    assert!(owner_body.contains("5678"));

    let admin_list = client
        .get(format!("{base}/console/api/tokens"))
        .bearer_auth(&admin_jwt)
        .send()
        .await?;
    assert_eq!(admin_list.status(), StatusCode::OK);
    let admin_json: Value = admin_list.json().await?;
    let rendered = admin_json.to_string();
    assert!(!rendered.contains(admin_key));
    assert!(!rendered.contains(user_key));
    assert!(rendered.contains("1234"));
    assert!(rendered.contains("5678"));

    Ok(())
}

#[tokio::test]
async fn sensitive_internal_mutations_require_internal_secret() -> anyhow::Result<()> {
    configure_security_env();
    let db = test_utils::make_isolated_db().await;
    let base = spawn_server(db).await?;
    let client = Client::new();
    let url = format!("{base}/console/internal/circuit-breaker/trip-all");

    let missing = client.post(&url).send().await?;
    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);

    let wrong = client
        .post(&url)
        .header("X-Internal-Secret", "wrong-secret")
        .send()
        .await?;
    assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);

    let allowed = client
        .post(&url)
        .header("X-Internal-Secret", INTERNAL_SECRET)
        .send()
        .await?;
    assert_eq!(
        allowed.status(),
        StatusCode::OK,
        "authorized internal operations must remain available"
    );

    Ok(())
}
