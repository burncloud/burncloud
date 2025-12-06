mod common;

use common::{setup_db, start_test_server};
use burncloud_database::sqlx;
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
        "#
    )
    .bind(id).bind(name).bind(base_url).bind(api_key.clone()).bind(match_path).bind(auth_type)
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

    let resp = client.post(&url)
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
