#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::disallowed_types,
    reason = "模型解析接口单元测试使用固定 DTO 和响应体断言"
)]

use super::*;
use axum::{routing::any, Router};
use burncloud_supply_model::{
    load_default_manifests, LocalModelUnsupportedReason, ModelManifest, Variant,
};
use serde_json::json;
use tokio::{net::TcpListener, task::JoinHandle};

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
            configured.min_ram_bytes,
            expected.min_ram_bytes.max(1),
            "{repository}"
        );
        assert_eq!(
            configured.min_vram_bytes,
            expected.min_vram_bytes.max(2),
            "{repository}"
        );
        assert_eq!(
            configured.disk_size_bytes,
            expected.disk_size_bytes.max(3),
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

async fn mock_repository(status: StatusCode, body: String) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("监听模拟仓库");
    let base = format!("http://{}", listener.local_addr().expect("模拟仓库地址"));
    let app = Router::new().fallback(any(move || {
        let body = body.clone();
        async move { (status, body) }
    }));
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("模拟仓库服务运行");
    });
    (base, task)
}

fn dynamic_manifest(variant: Variant) -> ModelManifest {
    ModelManifest {
        model_name: "Qwen/Qwen3-8B-GGUF".into(),
        description: "测试工件".into(),
        supported_backends: vec![LLAMA_CPP.into()],
        version: "main".into(),
        variants: vec![variant],
    }
}

fn dynamic_variant(size: u64, min_vram_bytes: u64) -> Variant {
    Variant {
        id: "Qwen3-8B-Q4_K_M.gguf".into(),
        model_size: None,
        quantization: "Q4_K_M".into(),
        backend: LLAMA_CPP.into(),
        min_ram_bytes: size,
        min_vram_bytes,
        disk_size_bytes: size,
        checksum: format!("sha256:{}", "a".repeat(64)),
        artifact_uri: "https://example.invalid/resolve/main/model.gguf".into(),
    }
}

#[tokio::test]
async fn modelscope_discovery_filters_invalid_artifacts_and_builds_direct_urls() {
    let hash = "A".repeat(64);
    let body = json!({
        "Code": 200,
        "Data": {
            "Files": [
                {"Path":"nested/model-Q4_K_M.gguf", "Size":1024, "Sha256":hash, "Type":"blob"},
                {"Path":"model-no-hash.gguf", "Size":1024, "Sha256":"", "Type":"blob"},
                {"Path":"model-invalid-hash.gguf", "Size":1024, "Sha256":"not-a-sha256", "Type":"blob"},
                {"Path":"model-zero.gguf", "Size":0, "Sha256":hash, "Type":"blob"},
                {"Path":"model.safetensors", "Size":1024, "Sha256":hash, "Type":"blob"},
                {"Path":"nested/../unsafe.gguf", "Size":1024, "Sha256":hash, "Type":"blob"},
                {"Path":"nested/model.gguf", "Size":1024, "Sha256":hash, "Type":"tree"}
            ]
        }
    });
    let (base, task) = mock_repository(StatusCode::OK, body.to_string()).await;
    let discovered = discover_manifest_at(
        &Client::new(),
        ModelSource::ModelScope,
        &base,
        "org/repo",
        "master",
        None,
    )
    .await
    .expect("ModelScope 工件发现");
    task.abort();

    let (manifest, sizes) = discovered;
    assert_eq!(manifest.variants.len(), 1);
    assert_eq!(manifest.variants[0].id, "nested/model-Q4_K_M.gguf");
    assert_eq!(
        manifest.variants[0].checksum,
        format!("sha256:{}", "a".repeat(64))
    );
    assert_eq!(
        manifest.variants[0].artifact_uri,
        "https://modelscope.cn/models/org/repo/resolve/master/nested/model-Q4_K_M.gguf"
    );
    assert!(manifest.variants[0].artifact_uri.contains("/resolve/"));
    assert!(!manifest.variants[0].artifact_uri.contains("/tree/"));
    assert_eq!(sizes, vec![("nested/model-Q4_K_M.gguf".into(), 1024)]);
}

#[tokio::test]
async fn huggingface_discovery_requires_lfs_sha_size_and_direct_resolve_url() {
    let hash = "B".repeat(64);
    let body = json!([
        {"type":"file", "path":"folder/model-Q5_K_M.gguf", "size":null, "lfs":{"oid":hash, "size":2048}},
        {"type":"file", "path":"missing-sha.gguf", "size":2048},
        {"type":"file", "path":"invalid-sha.gguf", "size":2048, "lfs":{"oid":"not-a-sha256"}},
        {"type":"file", "path":"zero.gguf", "size":0, "lfs":{"oid":hash}},
        {"type":"file", "path":"model.safetensors", "size":2048, "lfs":{"oid":hash}},
        {"type":"file", "path":"../unsafe.gguf", "size":2048, "lfs":{"oid":hash}},
        {"type":"directory", "path":"folder.gguf", "size":2048, "lfs":{"oid":hash}}
    ]);
    let (base, task) = mock_repository(StatusCode::OK, body.to_string()).await;
    let discovered = discover_manifest_at(
        &Client::new(),
        ModelSource::HfMirror,
        &base,
        "org/repo",
        "main",
        None,
    )
    .await
    .expect("Hugging Face 工件发现");
    task.abort();

    let (manifest, sizes) = discovered;
    assert_eq!(manifest.variants.len(), 1);
    assert_eq!(manifest.variants[0].id, "folder/model-Q5_K_M.gguf");
    assert_eq!(
        manifest.variants[0].checksum,
        format!("sha256:{}", "b".repeat(64))
    );
    assert_eq!(
        manifest.variants[0].artifact_uri,
        "https://hf-mirror.com/org/repo/resolve/main/folder/model-Q5_K_M.gguf"
    );
    assert_eq!(sizes, vec![("folder/model-Q5_K_M.gguf".into(), 2048)]);
}

#[tokio::test]
async fn discovery_preserves_upstream_not_found_and_unavailable_errors() {
    for (status, expected_not_found) in [
        (StatusCode::NOT_FOUND, true),
        (StatusCode::BAD_GATEWAY, false),
    ] {
        let (base, task) = mock_repository(status, "{}".into()).await;
        let result = discover_manifest_at(
            &Client::new(),
            ModelSource::HuggingFace,
            &base,
            "org/repo",
            "main",
            None,
        )
        .await;
        task.abort();
        if expected_not_found {
            assert!(matches!(result, Err(DiscoveryError::NotFound)));
        } else {
            assert!(matches!(result, Err(DiscoveryError::Unavailable(_))));
        }
    }
    assert!(matches!(
        repository_url(
            ModelSource::HuggingFace,
            "../repo",
            "main",
            "https://huggingface.co"
        ),
        Err(DiscoveryError::Invalid(_))
    ));
}

#[test]
fn builtin_resource_config_never_reduces_actual_artifact_requirements() {
    const GIB: u64 = 1024 * 1024 * 1024;
    let mut manifest = dynamic_manifest(dynamic_variant(20 * GIB, 8 * GIB));

    apply_builtin_resource_config(&mut manifest).expect("内置资源配置应有效");

    let variant = &manifest.variants[0];
    assert_eq!(variant.min_ram_bytes, 20 * GIB);
    assert_eq!(variant.min_vram_bytes, 8 * GIB);
    assert_eq!(variant.disk_size_bytes, 20 * GIB);
}

#[test]
fn preset_hardware_requirements_merge_minima_and_preserve_larger_artifact_values() {
    const GIB: u64 = 1024 * 1024 * 1024;
    let preset = crate::model_selection::presets::find("qwen3-8b")
        .expect("预置目录有效")
        .expect("Qwen 预置存在");
    let mut manifest = dynamic_manifest(dynamic_variant(1024, 1024));

    apply_preset_hardware_requirements(&mut manifest, Some(preset)).expect("合并预置门槛");
    let variant = &manifest.variants[0];
    assert_eq!(variant.min_vram_bytes, 6 * GIB);
    assert_eq!(variant.min_ram_bytes, 12 * GIB);
    assert_eq!(variant.disk_size_bytes, 6 * GIB);

    let mut larger = dynamic_manifest(dynamic_variant(20 * GIB, 24 * GIB));
    apply_preset_hardware_requirements(&mut larger, Some(preset)).expect("保留较大工件门槛");
    assert_eq!(larger.variants[0].min_vram_bytes, 24 * GIB);
    assert_eq!(larger.variants[0].min_ram_bytes, 20 * GIB);
    assert_eq!(larger.variants[0].disk_size_bytes, 20 * GIB);
}

#[test]
fn gibibyte_conversion_uses_checked_binary_units_and_rounds_up_fractional_bytes() {
    assert_eq!(gibibytes_to_bytes("1G").expect("整数 GiB"), 1_u64 << 30);
    assert_eq!(gibibytes_to_bytes("1.5G").expect("小数 GiB"), 3_u64 << 29);
    assert_eq!(gibibytes_to_bytes("0.1G").expect("向上取整"), 107_374_183);
    assert!(matches!(
        gibibytes_to_bytes("18446744073709551616G"),
        Err(DiscoveryError::Invalid(_))
    ));
    assert!(matches!(
        gibibytes_to_bytes("1T"),
        Err(DiscoveryError::Invalid(_))
    ));
}

#[test]
fn non_preset_manifest_keeps_its_existing_resource_requirements() {
    let mut manifest = dynamic_manifest(dynamic_variant(8192, 4096));
    let original = manifest.variants[0].clone();

    apply_preset_hardware_requirements(&mut manifest, None).expect("非预置路径兼容");

    assert_eq!(manifest.variants[0], original);
}

#[tokio::test]
async fn modelscope_success_without_file_metadata_is_unavailable() {
    let (base, task) =
        mock_repository(StatusCode::OK, json!({"Code":200,"Data":null}).to_string()).await;
    let result = discover_manifest_at(
        &Client::new(),
        ModelSource::ModelScope,
        &base,
        "org/repo",
        "master",
        None,
    )
    .await;
    task.abort();

    assert!(matches!(result, Err(DiscoveryError::Unavailable(_))));
}

#[test]
fn pending_identity_does_not_receive_preset_hardware_requirements() {
    let preset = crate::model_selection::presets::find("nsfw_image_detection")
        .expect("预置目录有效")
        .expect("待校验预置存在");
    let manifest = dynamic_manifest(dynamic_variant(1024, 1024));
    let original = manifest.variants[0].clone();
    let mut manifest = manifest;

    apply_preset_hardware_requirements(&mut manifest, Some(preset)).expect("待校验身份可查看");

    assert_eq!(manifest.variants[0], original);
}
