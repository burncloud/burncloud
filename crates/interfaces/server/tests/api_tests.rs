#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::let_and_return,
    clippy::disallowed_types,
    clippy::panic_in_result_fn,
    reason = "integration test: assertions are the failure signal, and clippy.toml's allow-panic-in-tests does not recognise #[tokio::test]"
)]

use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use burncloud_server::create_app;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;
use tower::ServiceExt;

mod test_utils;

#[tokio::test]
async fn test_api_health() -> anyhow::Result<()> {
    let db_arc = test_utils::make_isolated_db().await;
    let app = create_app(
        db_arc,
        false,
        test_utils::test_jwt_secret(),
        test_utils::test_internal_secret(),
    )
    .await?;

    let port = 4000_u16;
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .expect("Failed to bind test port");
        axum::serve(listener, app).await.expect("Server error");
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/console/api/channel", port);

    // Channel endpoints require authentication - expect 401 without token
    let resp = client.get(&url).send().await?;
    assert_eq!(resp.status(), 401);

    Ok(())
}

#[tokio::test]
async fn test_token_api_requires_auth() -> anyhow::Result<()> {
    let db_arc = test_utils::make_isolated_db().await;
    let app = create_app(
        db_arc,
        false,
        test_utils::test_jwt_secret(),
        test_utils::test_internal_secret(),
    )
    .await?;

    let port = 4001_u16;
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .expect("Failed to bind test port");
        axum::serve(listener, app).await.expect("Server error");
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let base_url = format!("http://localhost:{}/console/api/tokens", port);

    // Token API now requires authentication - expect 401 without token
    let resp = client.get(&base_url).send().await?;
    assert_eq!(
        resp.status(),
        401,
        "Token list should require authentication"
    );

    // POST should also require authentication
    let resp = client
        .post(&base_url)
        .json(&serde_json::json!({ "user_id": "test-user" }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        401,
        "Token create should require authentication"
    );

    Ok(())
}

#[tokio::test]
async fn test_log_api_requires_auth() -> anyhow::Result<()> {
    let db_arc = test_utils::make_isolated_db().await;
    let app = create_app(
        db_arc,
        false,
        test_utils::test_jwt_secret(),
        test_utils::test_internal_secret(),
    )
    .await?;

    let port = 4002_u16;
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .expect("Failed to bind test port");
        axum::serve(listener, app).await.expect("Server error");
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/console/api/logs", port);

    // Log API now requires authentication - expect 401 without token
    let resp = client.get(&url).send().await?;
    assert_eq!(resp.status(), 401, "Log API should require authentication");

    Ok(())
}

#[tokio::test]
async fn test_monitor_api_requires_auth() -> anyhow::Result<()> {
    let db_arc = test_utils::make_isolated_db().await;
    let app = create_app(
        db_arc,
        false,
        test_utils::test_jwt_secret(),
        test_utils::test_internal_secret(),
    )
    .await?;

    let port = 4003_u16;
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .expect("Failed to bind test port");
        axum::serve(listener, app).await.expect("Server error");
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/console/api/monitor", port);

    // Monitor API now requires authentication - expect 401 without token
    let resp = client.get(&url).send().await?;
    assert_eq!(
        resp.status(),
        401,
        "Monitor API should require authentication"
    );

    Ok(())
}

#[tokio::test]
async fn node_deployment_route_is_public_and_returns_json_for_missing_local_path(
) -> anyhow::Result<()> {
    let db_arc = test_utils::make_isolated_db().await;
    let app = create_app(
        db_arc,
        false,
        test_utils::test_jwt_secret(),
        test_utils::test_internal_secret(),
    )
    .await?;
    let model_path = std::env::temp_dir().join(format!(
        "burncloud-missing-model-{}-{}.gguf",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/console/api/node/deployments")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "model_path": model_path.to_string_lossy()
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    assert_eq!(response.status(), 400);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("application/json")
    );
    let body = to_bytes(response.into_body(), 1024).await?;
    assert!(
        String::from_utf8_lossy(&body).contains("model_path must point to an existing model file")
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/console/api/node/deployments/not-found")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), 404);

    Ok(())
}
