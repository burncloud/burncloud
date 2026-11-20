#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::disallowed_types,
    clippy::panic_in_result_fn,
    reason = "模型解析接口集成测试使用固定请求和响应断言"
)]

use axum::{
    body::Body,
    http::{HeaderMap, Request},
};
use burncloud_server::create_app;
use serde_json::Value;
use tower::ServiceExt;

mod test_utils;

async fn post_json(
    uri: &str,
    body: Value,
) -> anyhow::Result<(axum::http::StatusCode, HeaderMap, Value)> {
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
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
    Ok((status, headers, serde_json::from_slice(&bytes)?))
}

#[tokio::test]
async fn model_resolution_route_rejects_empty_model_before_external_discovery() -> anyhow::Result<()>
{
    let (status, headers, body) = post_json(
        "/api/node/models/resolve",
        serde_json::json!({
            "model": "  ",
            "source": "modelscope",
            "revision": "master"
        }),
    )
    .await?;

    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(headers["cache-control"], "no-store");
    assert_eq!(body["success"], false);
    assert_eq!(body["error"]["code"], "invalid_model");

    Ok(())
}

#[tokio::test]
async fn model_resolution_route_rejects_non_repository_model_identity() -> anyhow::Result<()> {
    let (status, _headers, body) = post_json(
        "/api/node/models/resolve",
        serde_json::json!({
            "model": "missing-model",
            "source": "modelscope",
            "revision": "master"
        }),
    )
    .await?;

    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(body["success"], false);
    assert_eq!(body["error"]["code"], "invalid_model");

    Ok(())
}

#[tokio::test]
async fn model_resolution_route_rejects_unknown_source() -> anyhow::Result<()> {
    let (status, _headers, body) = post_json(
        "/api/node/models/resolve",
        serde_json::json!({
            "model": "Qwen/Qwen3-8B",
            "source": "unknown",
            "revision": "master"
        }),
    )
    .await?;

    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(body["success"], false);
    assert_eq!(body["error"]["code"], "invalid_model");

    Ok(())
}
