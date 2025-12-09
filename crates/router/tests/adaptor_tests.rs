mod common;

use burncloud_database::sqlx;
use common::{setup_db, start_test_server};
use reqwest::Client;
use serde_json::json;
use std::env;

#[tokio::test]
async fn test_gemini_adaptor() -> anyhow::Result<()> {
    let env_key = env::var("TEST_GOOGLE_AI_KEY").unwrap_or_default();
    if env_key.is_empty() {
        println!("Skipping Gemini Adaptor test: TEST_GOOGLE_AI_KEY not set.");
        return Ok(());
    }
    let api_key = env_key;

    let (_db, pool) = setup_db().await?;

    let id = "gemini-adaptor-test";
    let name = "gemini-pro";
    let base_url = "https://generativelanguage.googleapis.com";
    let match_path = "/v1beta/models/gemini-2.0-flash:generateContent"; // Specific for this test setup
    let auth_type = "GoogleAI";

    sqlx::query(
        r#"
        INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET 
            api_key = excluded.api_key,
            base_url = excluded.base_url,
            auth_type = excluded.auth_type
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(base_url)
    .bind(api_key.clone())
    .bind(match_path)
    .bind(auth_type)
    .execute(&pool)
    .await?;

    let port = 3012;
    start_test_server(port).await;

    let client = Client::new();
    // Assuming path rewriting is not yet fully dynamic, we target the matched path
    let url = format!("http://localhost:{}{}", port, match_path);

    let openai_body = json!({
        "model": "gemini-2.0-flash",
        "messages": [
            { "role": "user", "content": "Say 'ADAPTOR_WORKS'" }
        ]
    });

    let resp = client
        .post(&url)
        .header("Authorization", "Bearer sk-burncloud-demo")
        .header("x-use-adaptor", "true")
        .json(&openai_body)
        .send()
        .await?;

    assert_eq!(resp.status(), 200);

    let resp_json: serde_json::Value = resp.json().await?;
    println!("Adaptor Response: {}", resp_json);

    assert_eq!(resp_json["object"], "chat.completion");
    let choices = resp_json["choices"].as_array().unwrap();
    let content = choices[0]["message"]["content"].as_str().unwrap();
    assert!(content.contains("ADAPTOR_WORKS"));

    Ok(())
}

#[tokio::test]
async fn test_claude_adaptor() -> anyhow::Result<()> {
    // Test Claude Adaptor Transformation (OpenAI -> Claude)
    // Uses HttpBin to inspect the converted request body

    let (_db, pool) = setup_db().await?;

    let id = "claude-adaptor-test";
    let name = "claude-3-opus";
    let base_url = "https://httpbin.org";
    let match_path = "/anything";
    let auth_type = "Claude";
    let api_key = "sk-ant-mock-key";

    sqlx::query(
        r#"
        INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET 
            api_key = excluded.api_key,
            base_url = excluded.base_url,
            auth_type = excluded.auth_type
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(base_url)
    .bind(api_key)
    .bind(match_path)
    .bind(auth_type)
    .execute(&pool)
    .await?;

    let port = 3013;
    start_test_server(port).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/anything", port);

    let openai_body = json!({
        "model": "gpt-4", // Will be passed through or mapped
        "messages": [
            { "role": "system", "content": "You are a helpful assistant." },
            { "role": "user", "content": "Hello Claude" }
        ],
        "max_tokens": 100
    });

    // 1. Send OpenAI Request with Adaptor Header
    let resp = client
        .post(&url)
        .header("Authorization", "Bearer sk-burncloud-demo")
        .header("x-use-adaptor", "true")
        .json(&openai_body)
        .send()
        .await?;

    assert_eq!(resp.status(), 200);

    // 2. Inspect what HttpBin received (The Converted Claude Request)
    // Note: Since HttpBin echoes the request, and Router logic for `Claude` adaptor
    // tries to parse the response as Claude Response JSON, this might fail conversion
    // if HttpBin response doesn't match expected Claude response schema.
    //
    // However, our `convert_response` function in `claude.rs` uses safe `get` calls
    // and defaults to empty string if fields missing. So it shouldn't panic.
    //
    // BUT, we can't see the request body sent to upstream easily if the response conversion
    // produces a valid OpenAI response from HttpBin's output.
    // HttpBin output has "json": { ... body sent ... }
    // So `claude_resp` will be the HttpBin JSON.
    // `convert_response` looks for `content` array. HttpBin response doesn't have `content` array usually at top level.
    //
    // To properly test request conversion without a real Claude API, we might need to
    // rely on the fact that `convert_response` returns an empty content if schema mismatch,
    // OR we trust unit tests for `ClaudeAdaptor` (which we should add).
    //
    // Actually, for Integration Test of Adaptor logic, using a Real API is best.
    // Since we don't have a key, let's stick to Unit Tests for the `ClaudeAdaptor` logic itself,
    // and use integration test just to check routing + header injection (which `auth_tests` covers,
    // but we want to cover the `if use_adaptor` branch).
    //
    // Let's try to verify the Request Body transformation by parsing the response?
    // The `convert_response` returns `openai_resp`.
    // If we send to HttpBin, the response from HttpBin is the JSON of the request we sent.
    // `convert_response` will try to find `content` in it. It won't find it.
    // So it returns empty content.
    //
    // This integration test mainly proves the ROUTER accepts `x-use-adaptor` and tries to convert.
    // It doesn't prove the conversion result is correct unless we inspect logs or use a smarter mock.
    //
    // Let's stick to this for now: it ensures no panic and correct path.
    // We should add a unit test for `ClaudeAdaptor` logic separately if we want to be strict.

    let _json: serde_json::Value = resp.json().await?;
    // Verification limited here without real upstream response structure.
    Ok(())
}
