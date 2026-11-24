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
    assert_eq!(
        value["components"]["schemas"]["ResolveModelRequest"]["properties"]["preset_id"]["type"],
        "string"
    );
    assert_eq!(
        value["components"]["schemas"]["ResolveReason"]["properties"]["code"]["enum"],
        serde_json::json!([
            "preset_identity_pending",
            "no_valid_gguf",
            "no_compatible_variant"
        ])
    );
    assert_eq!(
        operation["responses"]["502"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/ModelResolutionErrorResponse"
    );
    let error_codes = &value["components"]["schemas"]["ModelResolutionErrorResponse"]["properties"]
        ["error"]["properties"]["code"]["enum"];
    assert!(error_codes.as_array().is_some_and(|codes| {
        codes.iter().any(|code| code == "unknown_preset")
            && codes.iter().any(|code| code == "preset_identity_mismatch")
    }));
}

#[test]
fn preset_route_documents_the_full_directory_and_identity_metadata() {
    let value = serde_json::to_value(generate_openapi_spec()).expect("OpenAPI 应可序列化");
    let presets = &value["paths"]["/api/models/presets"]["get"];
    let schemas = &value["components"]["schemas"];

    assert_eq!(presets["operationId"], "listModelPresets");
    assert_eq!(presets["security"], serde_json::json!([]));
    let preset_array = &presets["responses"]["200"]["content"]["application/json"]["schema"];
    assert_eq!(preset_array["minItems"], 388);
    assert_eq!(preset_array["maxItems"], 388);
    assert_eq!(
        preset_array["items"]["$ref"],
        "#/components/schemas/CatalogPreset"
    );
    assert_eq!(
        schemas["CatalogPreset"]["properties"]["identity_status"]["enum"],
        serde_json::json!(["verified", "pending_validation"])
    );
    assert_eq!(
        schemas["PresetHardwareRequirements"]["required"],
        serde_json::json!([
            "vram_min",
            "ram_min",
            "disk_min",
            "vram_recommended",
            "ram_recommended",
            "disk_recommended"
        ])
    );
    assert!(
        schemas["CatalogPreset"]["properties"]["resource_url"]["description"]
            .as_str()
            .is_some_and(|description| description.contains("browsing only"))
    );
}

#[test]
fn external_search_route_documents_the_ndjson_contract() {
    let value = serde_json::to_value(generate_openapi_spec()).expect("OpenAPI 应可序列化");
    let search = &value["paths"]["/api/models/search"]["get"];

    assert_eq!(search["operationId"], "searchModels");
    assert_eq!(search["parameters"][0]["name"], "q");
    assert_eq!(
        search["responses"]["200"]["content"]["application/x-ndjson"]["schema"]["type"],
        "string"
    );
    assert_eq!(
        value["components"]["schemas"]["CatalogSearchLine"]["properties"]["type"]["enum"],
        serde_json::json!(["models", "unavailable", "failed"])
    );
}

#[test]
fn model_status_route_documents_verified_artifact_eligibility() {
    let value = serde_json::to_value(generate_openapi_spec()).expect("OpenAPI 应可序列化");
    let status = &value["paths"]["/api/models/status"]["get"];
    let schemas = &value["components"]["schemas"];

    assert_eq!(status["operationId"], "getModelStatus");
    assert_eq!(status["security"], serde_json::json!([]));
    assert_eq!(status["parameters"][0]["name"], "preset");
    assert_eq!(status["parameters"][1]["name"], "model");
    assert_eq!(status["parameters"][2]["name"], "source");
    assert_eq!(status["parameters"][3]["name"], "revision");
    assert_eq!(
        status["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/CatalogInspection"
    );
    assert_eq!(
        schemas["CatalogInspection"]["properties"]["downloadable"]["type"],
        "boolean"
    );
    assert_eq!(
        schemas["CatalogInspection"]["properties"]["unavailable_reason"]["allOf"][0]["$ref"],
        "#/components/schemas/UnavailableReason"
    );
    assert_eq!(
        schemas["CatalogVariant"]["properties"]["file_size_bytes"]["minimum"],
        1
    );
    assert!(
        schemas["CatalogVariant"]["properties"]["artifact_source"]["description"]
            .as_str()
            .is_some_and(|description| description.contains("Direct GGUF file URL"))
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
