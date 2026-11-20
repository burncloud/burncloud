#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::disallowed_types,
    reason = "硬件接口单元测试使用固定 DTO 和响应体断言"
)]
use super::*;
use burncloud_node_runtime::{AcceleratorKind, AcceleratorProfile, HardwareProfile};

#[test]
fn snapshot_uses_frontend_friendly_accelerator_kinds() {
    let snapshot = HardwareSnapshot::from(HardwareProfile {
        cpu: "Example CPU".into(),
        memory_bytes: 16 * 1024 * 1024 * 1024,
        disk_available_bytes: 32 * 1024 * 1024 * 1024,
        accelerators: vec![
            AcceleratorProfile {
                kind: AcceleratorKind::Nvidia,
                name: "NVIDIA Example".into(),
                memory_bytes: Some(8 * 1024 * 1024 * 1024),
            },
            AcceleratorProfile {
                kind: AcceleratorKind::Other,
                name: "Unknown Example".into(),
                memory_bytes: None,
            },
        ],
    });

    let value = serde_json::to_value(snapshot).expect("硬件快照应可序列化");
    assert_eq!(value["accelerators"][0]["kind"], "nvidia");
    assert_eq!(value["accelerators"][1]["kind"], "other");
    assert!(value["accelerators"][1]["memory_bytes"].is_null());
}

#[test]
fn hardware_errors_use_stable_public_codes() {
    let response =
        hardware_error_response(HardwareProbeError::UnsupportedPlatform("example".into()));
    assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
}

#[test]
fn snapshot_preserves_an_empty_accelerator_list() {
    let snapshot = HardwareSnapshot::from(HardwareProfile {
        cpu: "Example CPU".into(),
        memory_bytes: 16 * 1024 * 1024 * 1024,
        disk_available_bytes: 32 * 1024 * 1024 * 1024,
        accelerators: Vec::new(),
    });

    let value = serde_json::to_value(snapshot).expect("硬件快照应可序列化");
    assert_eq!(value["accelerators"], serde_json::json!([]));
}

#[tokio::test]
async fn hardware_errors_hide_internal_details_and_disable_caching() {
    let response =
        hardware_error_response(HardwareProbeError::DiskError("C:\\private\\path".into()));
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response
            .headers()
            .get(axum::http::header::CACHE_CONTROL)
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );

    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .expect("错误响应体应可读取");
    let value: serde_json::Value = serde_json::from_slice(&body).expect("错误响应应为 JSON");
    assert_eq!(value["error"]["code"], "hardware_probe_failed");
    assert_eq!(value["error"]["message"], "硬件检测失败");
    assert!(!body
        .windows(b"private".len())
        .any(|part| part == b"private"));
}
