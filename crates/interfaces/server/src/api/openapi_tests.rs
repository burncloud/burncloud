#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "OpenAPI 单元测试使用固定生成结果和断言辅助读取 JSON"
)]

use super::generate_openapi_spec;

#[test]
fn hardware_route_is_documented_as_public_with_nullable_memory() {
    let value = serde_json::to_value(generate_openapi_spec()).expect("OpenAPI 应可序列化");
    let operation = &value["paths"]["/api/node/hardware"]["get"];

    assert_eq!(operation["operationId"], "getNodeHardware");
    assert_eq!(operation["security"], serde_json::json!([]));
    assert_eq!(
        value["components"]["schemas"]["AcceleratorSnapshot"]["properties"]["memory_bytes"]
            ["nullable"],
        true
    );
}

#[test]
fn model_resolution_route_is_documented_as_public_for_external_catalog_models() {
    let value = serde_json::to_value(generate_openapi_spec()).expect("OpenAPI 应可序列化");
    let operation = &value["paths"]["/api/node/models/resolve"]["post"];

    assert_eq!(operation["operationId"], "resolveNodeModel");
    assert_eq!(operation["security"], serde_json::json!([]));
    assert!(value["paths"]["/api/node/models/manifests"].is_null());
    assert_eq!(
        value["components"]["schemas"]["ResolveModelRequest"]["required"],
        serde_json::json!(["model", "source", "revision"])
    );
    assert_eq!(
        value["components"]["schemas"]["ResolveModelRequest"]["properties"]["source"]["enum"],
        serde_json::json!(["modelscope", "hf-mirror", "huggingface"])
    );
    assert_eq!(
        value["components"]["schemas"]["ResolveModelData"]["properties"]["status"]["enum"],
        serde_json::json!(["resolved", "unsupported"])
    );
    assert_eq!(
        value["components"]["schemas"]["ModelCandidate"]["properties"]["checksum"]["type"],
        "string"
    );
}

#[test]
fn model_download_route_is_public_with_stable_schemas_and_errors() {
    let value = serde_json::to_value(generate_openapi_spec()).expect("OpenAPI 应可序列化");
    let operation = &value["paths"]["/api/node/models/download"]["post"];

    assert_eq!(operation["operationId"], "downloadNodeModel");
    assert_eq!(operation["security"], serde_json::json!([]));
    assert_eq!(
        value["components"]["schemas"]["DownloadModelRequest"]["required"],
        serde_json::json!([
            "model",
            "source",
            "revision",
            "variant_id",
            "artifact_uri",
            "checksum"
        ])
    );
    assert_eq!(
        value["components"]["schemas"]["DownloadModelData"]["properties"]["status"]["enum"],
        serde_json::json!(["verified"])
    );
    assert_eq!(
        value["components"]["schemas"]["ModelDownloadErrorResponse"]["properties"]["error"]
            ["properties"]["code"]["enum"],
        serde_json::json!([
            "invalid_download_request",
            "checksum_mismatch",
            "download_failed",
            "download_service_unavailable",
            "download_storage_failed"
        ])
    );
    for status in ["400", "422", "500", "502", "503"] {
        assert_eq!(
            operation["responses"][status]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/ModelDownloadErrorResponse"
        );
    }
}
