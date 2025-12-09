use burncloud_tests::TestClient;
use serde_json::json;
use uuid::Uuid;

use crate::common;

#[tokio::test]
async fn test_auth_invalid_token() {
    let base_url = common::spawn_app().await;
    let client = TestClient::new(&base_url).with_token("invalid-sk-123");
    let body = json!({
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": "hi"}]
    });

    // Expect 401
    let res = client
        .post_expect_error("/v1/chat/completions", &body, 401)
        .await;
    if let Err(e) = res {
        panic!("Invalid token test failed: {}", e);
    }
}

#[tokio::test]
async fn test_auth_no_token() {
    let base_url = common::spawn_app().await;
    let client = TestClient::new(&base_url); // No token
    let body = json!({
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": "hi"}]
    });

    // Expect 401
    let res = client
        .post_expect_error("/v1/chat/completions", &body, 401)
        .await;
    if let Err(ref e) = res {
        panic!("No token test failed: {}", e);
    }
}

#[tokio::test]
async fn test_user_flow() {
    let base_url = common::spawn_app().await;
    let client = TestClient::new(&base_url);

    let username = format!(
        "user_{}",
        Uuid::new_v4().to_string().replace("-", "")[..8].to_string()
    );
    let password = "Password123!";

    // 1. Register
    let reg_body = json!({
        "username": username,
        "password": password,
        "email": format!("{}@example.com", username)
    });

    let reg_res = client
        .post("/console/api/user/register", &reg_body)
        .await
        .expect("Register failed");
    assert_eq!(reg_res["success"], true, "Register response: {:?}", reg_res);
    let user_id = reg_res["data"]["id"]
        .as_str()
        .expect("No ID returned")
        .to_string();

    // 2. Login
    let login_body = json!({
        "username": username,
        "password": password
    });
    let login_res = client
        .post("/console/api/user/login", &login_body)
        .await
        .expect("Login failed");
    assert_eq!(
        login_res["success"], true,
        "Login response: {:?}",
        login_res
    );
    assert_eq!(login_res["data"]["username"], username);

    // 3. Login Invalid
    let invalid_login_body = json!({
        "username": username,
        "password": "WrongPassword"
    });
    let invalid_res = client
        .post("/console/api/user/login", &invalid_login_body)
        .await
        .expect("Login request failed");
    assert_eq!(invalid_res["success"], false);

    // 4. List Users (As Admin) - Assuming no auth needed for MVP or root token
    let list_res = client
        .get("/console/api/list_users")
        .await
        .expect("List users failed");
    assert_eq!(list_res["success"], true);
    let users = list_res["data"].as_array().expect("Users not array");
    assert!(users.iter().any(|u| u["username"] == username));

    // 5. Topup
    let topup_body = json!({
        "user_id": user_id,
        "amount": 100.0
    });
    let topup_res = client
        .post("/console/api/user/topup", &topup_body)
        .await
        .expect("Topup failed");
    assert_eq!(topup_res["success"], true);
    assert_eq!(topup_res["data"]["balance"], 110.0); // 10 (signup) + 100
}
