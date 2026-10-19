use burncloud_node_runtime::{AcceleratorKind, AcceleratorProfile, HardwareProfile};
use burncloud_supply_model::{
    LocalModelUnsupportedReason, ModelManifest, ModelResolutionOutcome, RealModelResolver, Variant,
};

fn variant(id: &str, backend: &str, min_vram_bytes: u64) -> Variant {
    Variant {
        id: id.into(),
        model_size: Some("7b".into()),
        quantization: "q4_k_m".into(),
        backend: backend.into(),
        min_ram_bytes: 8,
        min_vram_bytes,
        disk_size_bytes: 1,
        checksum: format!("sha256:{id}"),
        artifact_uri: format!("https://example.invalid/{id}.gguf"),
    }
}

fn hardware(vram_bytes: u64) -> HardwareProfile {
    HardwareProfile {
        cpu_threads: 8,
        memory_bytes: 8,
        disk_available_bytes: 100,
        accelerators: vec![AcceleratorProfile {
            kind: AcceleratorKind::Other,
            name: "test-accelerator".into(),
            memory_bytes: Some(vram_bytes),
        }],
    }
}

#[test]
fn resolve_with_capabilities_returns_the_variant_that_survives_both_filters() {
    let resolver = RealModelResolver::new(vec![ModelManifest {
        model_name: "deepseek-r1".into(),
        description: "test manifest".into(),
        supported_backends: vec!["llama.cpp".into()],
        version: "test".into(),
        variants: vec![
            variant("high-vram", "llama.cpp", 16),
            variant("unsupported-runtime", "vllm", 4),
            variant("compatible", "llama.cpp", 4),
        ],
    }]);

    let runtimes = vec!["llama.cpp".into()];
    let result = resolver
        .resolve_with_capabilities("deepseek-r1", &hardware(8), &runtimes)
        .unwrap();

    match result {
        ModelResolutionOutcome::Local(resolved) => {
            assert_eq!(resolved.runtime, "llama.cpp");
            assert!(resolved.artifact_source.ends_with("compatible.gguf"));
        }
        ModelResolutionOutcome::Unsupported(reason) => {
            panic!("expected a compatible variant, got {reason:?}");
        }
    }
}

#[test]
fn resolve_with_capabilities_reports_no_compatible_variant() {
    let resolver = RealModelResolver::new(vec![ModelManifest {
        model_name: "deepseek-r1".into(),
        description: "test manifest".into(),
        supported_backends: vec!["llama.cpp".into()],
        version: "test".into(),
        variants: vec![variant("high-vram", "llama.cpp", 16)],
    }]);

    let runtimes = vec!["llama.cpp".into()];
    let result = resolver
        .resolve_with_capabilities("deepseek-r1", &hardware(8), &runtimes)
        .unwrap();

    assert!(matches!(
        result,
        ModelResolutionOutcome::Unsupported(model) if matches!(
            model.reason,
            LocalModelUnsupportedReason::NoCompatibleVariant
        )
    ));
}
