#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::disallowed_types,
    clippy::panic_in_result_fn,
    reason = "接口集成测试使用断言辅助构造请求和读取测试响应"
)]

use axum::{body::Body, http::Request};
use burncloud_server::create_app;
use serde_json::Value;
use tower::ServiceExt;

mod test_utils;

#[tokio::test]
async fn hardware_snapshot_is_public_and_has_frontend_contract() -> anyhow::Result<()> {
    let app = create_app(
        test_utils::make_isolated_db().await,
        false,
        test_utils::test_jwt_secret(),
        test_utils::test_internal_secret(),
    )
    .await?;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/node/hardware")
                .header("accept", "application/json")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
    let payload: Value = serde_json::from_slice(&body)?;
    assert_eq!(payload["success"], true);
    assert!(payload["data"]["platform"].is_string());
    assert!(payload["data"]["arch"].is_string());
    assert!(payload["data"]["cpu"].is_string());
    assert!(payload["data"]["memory_bytes"].is_u64());
    assert!(payload["data"]["disk_available_bytes"].is_u64());
    assert!(payload["data"]["accelerators"].is_array());

    Ok(())
}
