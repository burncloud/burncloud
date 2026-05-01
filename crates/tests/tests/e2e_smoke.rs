//! E2E smoke test for v0.4 MVP — validates the 8-step user journey.
//!
//! Requires a running server. Set `E2E_BASE_URL` (default `http://localhost:3000`)
//! before running. The server must have a mock price for `gpt-4o-mini` inserted
//! (or `SKIP_INITIAL_PRICE_SYNC=0` with a working price source).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types)]

mod common;

fn base_url() -> String {
    std::env::var("E2E_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".into())
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("build client")
}

async fn get_admin_token() -> String {
    let url = format!("{}/api/auth/login", base_url());
    let body = serde_json::json!({
        "username": "e2e_qa_admin",
        "password": "QaTest169!"
    });
    let resp = client().post(&url).json(&body).send().await.expect("login request");
    let data: serde_json::Value = resp.json().await.expect("login response json");
    data["data"]["token"].as_str().expect("token").to_string()
}

#[tokio::test]
async fn e2e_smoke_v04() {
    let mut passed = 0u32;
    let mut failed = 0u32;

    // Step 1: Health check
    {
        let url = format!("{}/health", base_url());
        let resp = client().get(&url).send().await.expect("health request");
        let status = resp.status().as_u16();
        if status == 200 {
            passed += 1;
            eprintln!("  PASS  1_health");
        } else {
            failed += 1;
            eprintln!("  FAIL  1_health: expected 200, got {status}");
        }
    }

    // Step 2: First user registration — should get admin role
    {
        let url = format!("{}/api/auth/register", base_url());
        let body = serde_json::json!({
            "username": "e2e_qa_admin",
            "password": "QaTest169!",
            "email": "qa@e2e.test"
        });
        let resp = client().post(&url).json(&body).send().await.expect("register request");
        let data: serde_json::Value = resp.json().await.expect("register response json");
        let success = data["success"].as_bool().unwrap_or(false);
        let roles = data["data"]["roles"].as_array();
        let has_admin = roles.is_some_and(|r| r.iter().any(|v| v.as_str() == Some("admin")));
        let has_token = data["data"]["token"].as_str().is_some_and(|t| !t.is_empty());
        if success && has_admin && has_token {
            passed += 1;
            eprintln!("  PASS  2_first_user_admin");
        } else {
            failed += 1;
            eprintln!("  FAIL  2_first_user_admin: roles={:?}", data["data"]["roles"]);
        }
    }

    // Step 3: Login and get JWT
    {
        let token = get_admin_token().await;
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() == 3 {
            passed += 1;
            eprintln!("  PASS  3_login");
        } else {
            failed += 1;
            eprintln!("  FAIL  3_login: expected 3-part JWT, got {} parts", parts.len());
        }
    }

    // Step 4: Add upstream channel
    {
        let token = get_admin_token().await;
        let url = format!("{}/console/api/channel", base_url());
        let body = serde_json::json!({
            "name": "qa-burncloud-channel",
            "type": 1,
            "key": "sk-e8ll28SgjcRul7m27rTFo9qpBAlDmei9K4eOPMSeLZpJ9pkk",
            "base_url": "https://ai.burncloud.com",
            "models": "gpt-4o-mini",
            "group": "default",
            "weight": 1,
            "priority": 0
        });
        let resp = client()
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&body)
            .send()
            .await
            .expect("channel request");
        let data: serde_json::Value = resp.json().await.expect("channel response json");
        let success = data["success"].as_bool().unwrap_or(false);
        if success {
            passed += 1;
            eprintln!("  PASS  4_add_channel");
        } else {
            failed += 1;
            eprintln!("  FAIL  4_add_channel: {data}");
        }
    }

    // Step 5: LLM request through gateway (using JWT token)
    {
        let token = get_admin_token().await;
        let url = format!("{}/v1/chat/completions", base_url());
        let body = serde_json::json!({
            "model": "gpt-4o-mini",
            "messages": [{"role": "user", "content": "Say hello in 3 words"}],
            "max_tokens": 10
        });
        let resp = client()
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&body)
            .send()
            .await
            .expect("llm request");
        let data: serde_json::Value = resp.json().await.expect("llm response json");
        let has_choices = data["choices"].as_array().is_some_and(|c| !c.is_empty());
        let has_content = data["choices"][0]["message"]["content"]
            .as_str()
            .is_some_and(|c| !c.is_empty());
        let has_usage = data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) > 0;
        if has_choices && has_content && has_usage {
            passed += 1;
            eprintln!("  PASS  5_llm_request");
        } else {
            failed += 1;
            eprintln!("  FAIL  5_llm_request: {data}");
        }
    }

    // Step 6: Billing summary
    {
        let token = get_admin_token().await;
        let url = format!("{}/api/billing/summary", base_url());
        let resp = client()
            .get(&url)
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .expect("billing request");
        let data: serde_json::Value = resp.json().await.expect("billing response json");
        let success = data["success"].as_bool().unwrap_or(false);
        let models = data["data"]["models"].as_array();
        let has_models = models.is_some_and(|m| !m.is_empty());
        let first_requests = models
            .and_then(|m| m.first())
            .and_then(|m| m["requests"].as_u64())
            .unwrap_or(0);
        let total_cost = data["data"]["total_cost_usd"].as_f64().unwrap_or(-1.0);
        let has_pre_migration = data["data"].get("pre_migration_requests").is_some();
        if success && has_models && first_requests >= 1 && total_cost >= 0.0 && has_pre_migration {
            passed += 1;
            eprintln!("  PASS  6_billing_summary");
        } else {
            failed += 1;
            eprintln!("  FAIL  6_billing_summary: {data}");
        }
    }

    // Step 7: Dashboard returns HTML
    {
        let url = base_url();
        let resp = client()
            .get(&url)
            .header("Accept", "text/html")
            .send()
            .await
            .expect("dashboard request");
        let status = resp.status().as_u16();
        let ct = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let is_html = ct.contains("text/html");
        if status == 200 && is_html {
            passed += 1;
            eprintln!("  PASS  7_dashboard_html");
        } else {
            failed += 1;
            eprintln!("  FAIL  7_dashboard_html: expected HTML 200, got {status} {ct}");
        }
    }

    // Step 8: Second user gets "user" role (not admin)
    {
        let url = format!("{}/api/auth/register", base_url());
        let body = serde_json::json!({
            "username": "e2e_qa_user",
            "password": "QaTest169!",
            "email": "user@e2e.test"
        });
        let resp = client().post(&url).json(&body).send().await.expect("register2 request");
        let data: serde_json::Value = resp.json().await.expect("register2 response json");
        let success = data["success"].as_bool().unwrap_or(false);
        let roles = data["data"]["roles"].as_array();
        let has_admin = roles.is_some_and(|r| r.iter().any(|v| v.as_str() == Some("admin")));
        let has_user = roles.is_some_and(|r| r.iter().any(|v| v.as_str() == Some("user")));
        if success && !has_admin && has_user {
            passed += 1;
            eprintln!("  PASS  8_second_user");
        } else {
            failed += 1;
            eprintln!("  FAIL  8_second_user: roles={:?}", data["data"]["roles"]);
        }
    }

    let total = passed + failed;
    eprintln!("\nPassed: {passed}/{total}");

    assert_eq!(failed, 0, "{failed} e2e step(s) failed");
}
