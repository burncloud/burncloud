use burncloud_node_runtime::{AcceleratorKind, AcceleratorProfile, HardwareProfile};
use burncloud_supply_model::{
    CandidateResolutionOutcome, LocalModelUnsupportedReason, ModelManifest, RealModelResolver,
    Variant,
};

fn variant(id: &str, backend: &str, min_vram_bytes: u64) -> Variant {
    variant_with_disk(id, backend, min_vram_bytes, 1)
}

fn variant_with_disk(
    id: &str,
    backend: &str,
    min_vram_bytes: u64,
    disk_size_bytes: u64,
) -> Variant {
    Variant {
        id: id.into(),
        model_size: Some("7b".into()),
        quantization: "q4_k_m".into(),
        backend: backend.into(),
        min_ram_bytes: 8,
        min_vram_bytes,
        disk_size_bytes,
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
fn resolve_with_capabilities_returns_all_variants_that_survive_both_filters() {
    let resolver = RealModelResolver::new(vec![ModelManifest {
        model_name: "deepseek-r1".into(),
        description: "test manifest".into(),
        supported_backends: vec!["llama.cpp".into()],
        version: "test".into(),
        variants: vec![
            variant("high-vram", "llama.cpp", 16),
            variant("unsupported-runtime", "vllm", 4),
            variant("compatible-first", "llama.cpp", 4),
            variant("compatible-second", "llama.cpp", 4),
        ],
    }]);

    let runtimes = vec!["llama.cpp".into()];
    let result = resolver
        .resolve_with_capabilities("deepseek-r1", &hardware(8), &runtimes)
        .unwrap();

    match result {
        CandidateResolutionOutcome::Candidates(candidates) => {
            assert_eq!(candidates.len(), 2);
            assert_eq!(candidates[0].model_name, "deepseek-r1");
            assert_eq!(candidates[0].quantization, "q4_k_m");
            assert_eq!(candidates[0].min_ram_bytes, 8);
            assert_eq!(candidates[0].min_vram_bytes, 4);
            assert_eq!(candidates[0].disk_size_bytes, 1);
            assert_eq!(candidates[0].runtime, "llama.cpp");
            assert!(candidates[0]
                .artifact_uri
                .ends_with("compatible-first.gguf"));
            assert_eq!(candidates[0].checksum, "sha256:compatible-first");
            assert!(candidates[1]
                .artifact_uri
                .ends_with("compatible-second.gguf"));
        }
        CandidateResolutionOutcome::Unsupported(reason) => {
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
        CandidateResolutionOutcome::Unsupported(model) if matches!(
            model.reason,
            LocalModelUnsupportedReason::NoCompatibleVariant
        )
    ));
}

#[test]
fn resolve_with_capabilities_filters_by_disk_space() {
    let resolver = RealModelResolver::new(vec![ModelManifest {
        model_name: "deepseek-r1".into(),
        description: "test manifest".into(),
        supported_backends: vec!["llama.cpp".into()],
        version: "test".into(),
        variants: vec![
            variant_with_disk("exact-fit", "llama.cpp", 4, 100),
            variant_with_disk("too-large", "llama.cpp", 4, 101),
        ],
    }]);

    let result = resolver
        .resolve_with_capabilities(
            "deepseek-r1",
            &HardwareProfile {
                cpu_threads: 8,
                memory_bytes: 8,
                disk_available_bytes: 100,
                accelerators: vec![AcceleratorProfile {
                    kind: AcceleratorKind::Other,
                    name: "test-accelerator".into(),
                    memory_bytes: Some(4),
                }],
            },
            &["llama.cpp".into()],
        )
        .unwrap();

    match result {
        CandidateResolutionOutcome::Candidates(candidates) => {
            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0].model_name, "deepseek-r1");
            assert_eq!(candidates[0].disk_size_bytes, 100);
        }
        CandidateResolutionOutcome::Unsupported(reason) => {
            panic!("expected the exact-fit variant, got {reason:?}");
        }
    }
}

#[test]
fn resolve_with_capabilities_reports_unsupported_when_disk_is_insufficient() {
    let resolver = RealModelResolver::new(vec![ModelManifest {
        model_name: "deepseek-r1".into(),
        description: "test manifest".into(),
        supported_backends: vec!["llama.cpp".into()],
        version: "test".into(),
        variants: vec![variant_with_disk("needs-disk", "llama.cpp", 4, 101)],
    }]);

    let result = resolver
        .resolve_with_capabilities("deepseek-r1", &hardware(4), &["llama.cpp".into()])
        .unwrap();

    assert!(matches!(
        result,
        CandidateResolutionOutcome::Unsupported(model)
            if matches!(model.reason, LocalModelUnsupportedReason::NoCompatibleVariant)
    ));
}
