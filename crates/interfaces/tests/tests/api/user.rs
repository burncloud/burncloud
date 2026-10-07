#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::let_unit_value,
    clippy::redundant_pattern,
    clippy::manual_is_multiple_of,
    clippy::let_and_return,
    clippy::to_string_trait_impl,
    clippy::to_string_in_format_args,
    clippy::redundant_pattern_matching,
    reason = "integration-test code: panicking on fixture failures is the intended signal, and fixtures use plain Value formatting and patterns"
)]
use crate::common::{admin_client, spawn_app};
use burncloud_tests::TestClient;
use serde_json::json;

#[tokio::test]
async fn test_user_management_lifecycle() -> anyhow::Result<()> {
    // 1. Start Server
    let base_url = spawn_app().await;
    let admin = admin_client(&base_url).await;
    let public = TestClient::new(&base_url);

    // 2. Register User through the administrator-only management endpoint.
    let username = format!("testuser-{}", uuid::Uuid::new_v4());
    let password = "password123";
    let body = json!({
        "username": username,
        "password": password,
        "email": format!("{}@example.com", username)
    });

    let res = admin.post("/console/api/user/register", &body).await?;
    assert!(res["success"].as_bool().unwrap_or(false));
    let _user_id = res["data"]["id"].as_str().unwrap();

    // 3. Login
    let login_body = json!({
        "username": username,
        "password": password
    });
    // Login itself has a public auth endpoint; the returned JWT is the
    // management-plane credential for this user.
    let login_res = public.post("/api/auth/login", &login_body).await?;
    assert!(login_res["success"].as_bool().unwrap_or(false));
    assert_eq!(login_res["data"]["username"], username);
    assert!(!login_res["data"]["token"].is_null());

    // 4. List Users (Should contain the new user). This route is admin-only.
    let list_res = admin.get("/console/api/list_users").await?;
    assert!(list_res["success"].as_bool().unwrap_or(false));
    let users = list_res["data"].as_array().unwrap();

    let found = users.iter().any(|u| u["username"] == username);
    assert!(found, "Newly registered user not found in list");

    Ok(())
}
