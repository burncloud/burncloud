use burncloud_node_runtime::{HardwareProbe, HardwareProfile, SystemHardwareProbe};
use burncloud_supply_model::{
    LocalModelUnsupportedReason, ModelManifest, ModelResolutionError, ModelResolutionOutcome,
    ModelResolutionRequest, ModelResolver, RealModelResolver, Variant,
};

fn variant(id: &str, backend: &str, ram: u64, vram: u64) -> Variant {
    Variant {
        id: id.into(),
        model_size: None,
        quantization: "q4_k_m".into(),
        backend: backend.into(),
        min_ram_bytes: ram,
        min_vram_bytes: vram,
        disk_size_bytes: 1,
        checksum: format!("sha256:{}", "a".repeat(64)),
        artifact_uri: format!("https://example.invalid/{id}.gguf"),
    }
}

fn manifest(name: &str, variants: Vec<Variant>) -> ModelManifest {
    ModelManifest {
        model_name: name.into(),
        description: "兼容性测试用模型要求".into(),
        supported_backends: vec!["llama.cpp".into()],
        version: "1".into(),
        variants,
    }
}

fn maximum_vram(hardware: &HardwareProfile) -> u64 {
    hardware
        .accelerators
        .iter()
        .filter_map(|accelerator| accelerator.memory_bytes)
        .max()
        .unwrap_or(0)
}

#[test]
fn constructor_extracts_variants_in_manifest_order() {
    let resolver = RealModelResolver::new(vec![
        manifest("a", vec![variant("a-q4", "llama.cpp", 1, 0)]),
        manifest("b", vec![variant("b-q4", "llama.cpp", 1, 0)]),
    ]);
    assert_eq!(resolver.manifests().len(), 2);
    assert_eq!(resolver.variants()[0].id, "a-q4");
    assert_eq!(resolver.variants()[1].id, "b-q4");
}

#[tokio::test]
async fn hardware_filter_uses_real_ram_and_maximum_device_vram_at_the_boundary() {
    let hardware = SystemHardwareProbe::new()
        .inspect()
        .await
        .expect("真实探测应成功");
    let vram = maximum_vram(&hardware);
    let variants = vec![
        variant("fits", "llama.cpp", hardware.memory_bytes, vram),
        variant("ram-too-large", "llama.cpp", hardware.memory_bytes + 1, 0),
        variant("vram-too-large", "llama.cpp", 0, vram + 1),
    ];
    let filtered = RealModelResolver::filter_by_hardware(&variants, &hardware);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "fits");
}

#[test]
fn runtime_filter_keeps_only_allowed_backends() {
    let variants = vec![
        variant("llama", "llama.cpp", 1, 0),
        variant("vllm", "vllm", 1, 0),
    ];
    let filtered = RealModelResolver::filter_by_runtime(&variants, &["llama.cpp".into()]);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "llama");
}

#[tokio::test]
async fn resolve_uses_the_selected_model_real_hardware_and_manifest_backend() {
    let hardware = SystemHardwareProbe::new()
        .inspect()
        .await
        .expect("真实探测应成功");
    let vram = maximum_vram(&hardware);
    let accepted = variant("accepted", "llama.cpp", hardware.memory_bytes, vram);
    let resolver = RealModelResolver::new(vec![
        manifest(
            "other-model",
            vec![variant("wrong-model", "llama.cpp", 0, 0)],
        ),
        manifest(
            "selected-model",
            vec![
                variant("needs-more-ram", "llama.cpp", hardware.memory_bytes + 1, 0),
                variant("needs-more-vram", "llama.cpp", 0, vram + 1),
                variant("unsupported-backend", "vllm", 0, 0),
                accepted.clone(),
                variant("later-match", "llama.cpp", 0, 0),
            ],
        ),
    ]);
    let outcome = resolver
        .resolve(ModelResolutionRequest {
            model: " selected-model ".into(),
            hardware,
        })
        .await
        .expect("解析应成功");
    let ModelResolutionOutcome::Local(resolved) = outcome else {
        panic!("真实资源满足清单条件时应保留兼容变体");
    };
    assert_eq!(resolved.model, "selected-model");
    assert_eq!(resolved.artifact_source, accepted.artifact_uri);
    assert_eq!(
        resolved.artifact_digest.as_deref(),
        Some(accepted.checksum.as_str())
    );
    assert_eq!(resolved.runtime, "llama.cpp");
    assert_eq!(resolved.runtime_version, None);
}

#[tokio::test]
async fn resolve_reports_unsupported_for_requirements_exceeding_real_hardware() {
    let hardware = SystemHardwareProbe::new()
        .inspect()
        .await
        .expect("真实探测应成功");
    for candidate in [
        variant("too-much-ram", "llama.cpp", hardware.memory_bytes + 1, 0),
        variant("too-much-vram", "llama.cpp", 0, maximum_vram(&hardware) + 1),
        variant("wrong-backend", "vllm", 0, 0),
    ] {
        let resolver = RealModelResolver::new(vec![manifest("selected", vec![candidate])]);
        let outcome = resolver
            .resolve(ModelResolutionRequest {
                model: "selected".into(),
                hardware: hardware.clone(),
            })
            .await
            .expect("不兼容是正常判断结果");
        assert!(
            matches!(outcome, ModelResolutionOutcome::Unsupported(reason)
            if reason.reason == LocalModelUnsupportedReason::NoCompatibleVariant)
        );
    }
}

#[tokio::test]
async fn resolve_rejects_empty_or_unknown_models() {
    let hardware = SystemHardwareProbe::new()
        .inspect()
        .await
        .expect("真实探测应成功");
    let resolver = RealModelResolver::new(Vec::new());
    for model in ["  ", "not-in-catalog"] {
        let result = resolver
            .resolve(ModelResolutionRequest {
                model: model.into(),
                hardware: hardware.clone(),
            })
            .await;
        assert!(matches!(
            result,
            Err(ModelResolutionError::ResolutionFailed(_))
        ));
    }
}
