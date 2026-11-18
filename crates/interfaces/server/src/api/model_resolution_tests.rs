#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::disallowed_types,
    reason = "模型解析接口单元测试使用固定 DTO 和响应体断言"
)]

use super::*;
use burncloud_supply_model::{load_default_manifests, LocalModelUnsupportedReason};

#[test]
fn candidate_serializes_frontend_contract() {
    let candidate = ModelCandidate::from(ResolvedModelCandidate {
        model_name: "demo".into(),
        variant_id: "demo-q4".into(),
        quantization: "Q4_K_M".into(),
        min_ram_bytes: 8,
        min_vram_bytes: 4,
        disk_size_bytes: 3,
        runtime: "llama.cpp".into(),
        artifact_uri: "https://example.invalid/demo.gguf".into(),
        checksum: "sha256:example".into(),
    });

    let value = serde_json::to_value(candidate).expect("候选项应可序列化");
    assert_eq!(value["model_name"], "demo");
    assert_eq!(value["variant_id"], "demo-q4");
    assert_eq!(value["quantization"], "Q4_K_M");
    assert_eq!(value["runtime"], "llama.cpp");
}

#[test]
fn default_manifest_contains_shared_models_for_model_selection() {
    let manifests = load_default_manifests().expect("默认 Manifest 应有效");
    assert!(!manifests.is_empty());
}

#[test]
fn unsupported_reason_uses_stable_frontend_code() {
    let reason = resolve_reason(&LocalModelUnsupportedReason::NoCompatibleVariant);
    let value = serde_json::to_value(reason).expect("不支持原因应可序列化");

    assert_eq!(value["code"], "no_compatible_variant");
    assert_eq!(value["message"], "当前模型仓库没有满足当前硬件的 GGUF 版本");
}

#[test]
fn dynamic_artifact_building_preserves_path_checksum_and_source() {
    let artifact = DiscoveredArtifact {
        path: "nested/demo-Q4_K_M.gguf".into(),
        size: 8 * 1024,
        sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
    };
    let variant = build_variant(ModelSource::ModelScope, "Qwen/Qwen3-8B", "master", artifact)
        .expect("动态工件应可构造 Variant");

    assert_eq!(variant.id, "nested/demo-Q4_K_M.gguf");
    assert_eq!(variant.quantization, "Q4_K_M");
    assert_eq!(variant.backend, LLAMA_CPP);
    assert_eq!(variant.min_ram_bytes, 8 * 1024);
    assert_eq!(variant.min_vram_bytes, 0);
    assert_eq!(variant.disk_size_bytes, 8 * 1024);
    assert_eq!(
        variant.checksum,
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert_eq!(
        variant.artifact_uri,
        "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/nested/demo-Q4_K_M.gguf"
    );
}

#[test]
fn discovery_helpers_filter_paths_and_normalize_hashes() {
    assert!(is_gguf("model.GgUf"));
    assert!(!is_gguf("model.safetensors"));
    assert_eq!(quantization_from_path("demo-q8_0.gguf"), "Q8_0");
    assert_eq!(quantization_from_path("demo.gguf"), "GGUF");
    assert_eq!(
        normalize_sha256("sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into())
    );
    assert!(normalize_sha256("not-a-sha256").is_none());
    assert!(valid_model_id("owner/model-name"));
    assert!(!valid_model_id("owner/model/name"));
    assert!(!valid_path_segment(".."));
    assert!(!valid_path_segment("model\\file.gguf"));
}

#[test]
fn repository_urls_encode_external_model_identity() {
    let modelscope = repository_url(
        ModelSource::ModelScope,
        "Qwen/Qwen3-8B",
        "master",
        RepositoryOperation::Files,
    )
    .expect("ModelScope 地址");
    assert_eq!(
        modelscope.as_str(),
        "https://modelscope.cn/api/v1/models/Qwen/Qwen3-8B/repo/files?Revision=master&Recursive=true"
    );

    let huggingface = repository_url(
        ModelSource::HuggingFace,
        "TheBloke/TinyLlama",
        "main",
        RepositoryOperation::Files,
    )
    .expect("Hugging Face 地址");
    assert_eq!(
        huggingface.as_str(),
        "https://huggingface.co/api/models/TheBloke/TinyLlama/tree/main?recursive=true"
    );
}

#[tokio::test]
async fn error_response_is_json_and_not_cacheable() {
    let response = error_response(
        StatusCode::NOT_FOUND,
        "model_repository_not_found",
        "模型仓库或指定修订不存在",
    );
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response
            .headers()
            .get(header::CACHE_CONTROL)
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );

    let body = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .expect("错误响应体应可读取");
    let value: serde_json::Value = serde_json::from_slice(&body).expect("错误响应应为 JSON");
    assert_eq!(value["success"], false);
    assert_eq!(value["error"]["code"], "model_repository_not_found");
}
