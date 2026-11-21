#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::disallowed_types,
    reason = "模型解析接口单元测试使用固定 DTO 和响应体断言"
)]

use super::*;
use burncloud_supply_model::{
    load_default_manifests, LocalModelUnsupportedReason, ModelManifest, Variant,
};

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
fn discovered_qwen_q4_uses_builtin_resource_budget() {
    let mut manifest = ModelManifest {
        model_name: "Qwen/Qwen3-8B-GGUF".into(),
        description: "动态发现".into(),
        supported_backends: vec![LLAMA_CPP.into()],
        version: "master".into(),
        variants: vec![Variant {
            id: "Qwen3-8B-Q4_K_M.gguf".into(),
            model_size: None,
            quantization: "Q4_K_M".into(),
            backend: LLAMA_CPP.into(),
            min_ram_bytes: 1,
            min_vram_bytes: 0,
            disk_size_bytes: 1,
            checksum: "sha256:dynamic".into(),
            artifact_uri: "https://example.invalid/qwen.gguf".into(),
        }],
    };

    apply_builtin_resource_config(&mut manifest).expect("内置资源配置应有效");

    let variant = &manifest.variants[0];
    assert_eq!(variant.min_ram_bytes, 16 * 1024 * 1024 * 1024);
    assert_eq!(variant.min_vram_bytes, 6 * 1024 * 1024 * 1024);
    assert_eq!(variant.disk_size_bytes, 6 * 1024 * 1024 * 1024);
    assert_eq!(variant.checksum, "sha256:dynamic");
    assert_eq!(variant.artifact_uri, "https://example.invalid/qwen.gguf");
}

#[test]
fn all_builtin_gguf_models_use_their_resource_budget() {
    let builtin_manifests = load_default_manifests().expect("默认 Manifest 应有效");
    let preset_manifests = builtin_manifests
        .iter()
        .filter(|manifest| manifest.version.starts_with("preset-"))
        .collect::<Vec<_>>();
    assert_eq!(preset_manifests.len(), 17);

    for builtin in preset_manifests {
        let repository = artifact_repository_key(&builtin.variants[0].artifact_uri)
            .expect("内置 GGUF 仓库地址应可解析");
        let mut discovered = ModelManifest {
            model_name: format!("owner/{repository}"),
            description: "动态发现".into(),
            supported_backends: vec![LLAMA_CPP.into()],
            version: "master".into(),
            variants: vec![Variant {
                id: format!("{repository}.gguf"),
                model_size: None,
                quantization: "GGUF".into(),
                backend: LLAMA_CPP.into(),
                min_ram_bytes: 1,
                min_vram_bytes: 2,
                disk_size_bytes: 3,
                checksum: "sha256:dynamic".into(),
                artifact_uri: "https://example.invalid/dynamic.gguf".into(),
            }],
        };

        apply_builtin_resource_config(&mut discovered).expect("内置资源配置应有效");
        let configured = &discovered.variants[0];
        let expected = &builtin.variants[0];
        assert_eq!(
            configured.min_ram_bytes, expected.min_ram_bytes,
            "{repository}"
        );
        assert_eq!(
            configured.min_vram_bytes, expected.min_vram_bytes,
            "{repository}"
        );
        assert_eq!(
            configured.disk_size_bytes, expected.disk_size_bytes,
            "{repository}"
        );
        assert_eq!(configured.checksum, "sha256:dynamic");
        assert_eq!(
            configured.artifact_uri,
            "https://example.invalid/dynamic.gguf"
        );
    }
}

#[test]
fn quantization_matching_accepts_builtin_aliases() {
    assert!(quantization_matches("Q4/Q5", "Q4_K_M"));
    assert!(quantization_matches("4-bit", "Q4_K_M"));
    assert!(quantization_matches("IQ1_S", "UD-IQ1_S"));
    assert!(quantization_matches("MXFP4", "MXFP4"));
    assert!(!quantization_matches("Q4", "Q8_0"));
}

#[test]
fn discovery_helpers_filter_paths_and_normalize_hashes() {
    assert!(is_gguf("model.GgUf"));
    assert!(!is_gguf("model.safetensors"));
    assert_eq!(quantization_from_path("demo-q8_0.gguf"), "Q8_0");
    assert_eq!(quantization_from_path("demo-mxfp4.gguf"), "MXFP4");
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
        ModelSource::ModelScope.base_url(),
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
        ModelSource::HuggingFace.base_url(),
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
