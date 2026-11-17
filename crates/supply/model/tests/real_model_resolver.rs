use burncloud_node_runtime::{AcceleratorKind, AcceleratorProfile, HardwareProfile};
use burncloud_supply_model::{
    CandidateResolutionOutcome, LocalModelUnsupportedReason, ModelManifest, ModelResolutionError,
    ModelResolutionOutcome, ModelResolutionRequest, ModelResolver, RealModelResolver, Variant,
};

fn variant(
    id: &str,
    backend: &str,
    min_ram_bytes: u64,
    min_vram_bytes: u64,
    disk_size_bytes: u64,
) -> Variant {
    Variant {
        id: id.into(),
        model_size: Some("7b".into()),
        quantization: "q4_k_m".into(),
        backend: backend.into(),
        min_ram_bytes,
        min_vram_bytes,
        disk_size_bytes,
        checksum: format!("sha256:{id}"),
        artifact_uri: format!("https://example.invalid/{id}.gguf"),
    }
}

fn manifest_with_backends(
    name: &str,
    supported_backends: &[&str],
    variants: Vec<Variant>,
) -> ModelManifest {
    ModelManifest {
        model_name: name.into(),
        description: "test manifest".into(),
        supported_backends: supported_backends
            .iter()
            .map(|backend| (*backend).into())
            .collect(),
        version: "test".into(),
        variants,
    }
}

fn manifest(name: &str, variants: Vec<Variant>) -> ModelManifest {
    manifest_with_backends(name, &["llama.cpp", "vllm"], variants)
}

fn hardware(vram_bytes: u64) -> HardwareProfile {
    HardwareProfile {
        cpu: "test-cpu".into(),
        memory_bytes: 8,
        disk_available_bytes: 100,
        accelerators: vec![AcceleratorProfile {
            kind: AcceleratorKind::Other,
            name: "test-accelerator".into(),
            memory_bytes: Some(vram_bytes),
        }],
    }
}

fn resolution_request(model: &str, hardware: HardwareProfile) -> ModelResolutionRequest {
    ModelResolutionRequest {
        model: model.into(),
        hardware,
    }
}

#[test]
fn constructor_extracts_all_variants_from_all_manifests() {
    let resolver = RealModelResolver::new(vec![
        manifest(
            "model-a",
            vec![
                variant("a-q4", "llama.cpp", 8, 1, 1),
                variant("a-q8", "llama.cpp", 8, 1, 1),
            ],
        ),
        manifest_with_backends("model-b", &["vllm"], vec![variant("b-q4", "vllm", 8, 1, 1)]),
    ])
    .unwrap();

    assert_eq!(resolver.manifests().len(), 2);
    assert_eq!(
        resolver
            .variants()
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        ["a-q4", "a-q8", "b-q4"]
    );
}

#[test]
fn constructor_rejects_variant_backend_not_declared_in_manifest() {
    let result = RealModelResolver::new(vec![manifest_with_backends(
        "deepseek-r1",
        &["llama.cpp"],
        vec![variant("vllm-variant", "vllm", 8, 1, 1)],
    )]);

    assert!(matches!(
        result,
        Err(ModelResolutionError::ResolutionFailed(message))
            if message.contains("MANIFEST_BACKEND_MISMATCH")
                && message.contains("vllm")
    ));
}

#[test]
fn hardware_filter_keeps_variant_when_ram_and_vram_are_sufficient() {
    let variants = vec![variant("compatible", "llama.cpp", 8, 6, 1)];
    let filtered = RealModelResolver::filter_by_hardware(&variants, &hardware(8));

    assert_eq!(
        filtered
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        ["compatible"]
    );
}

#[test]
fn hardware_filter_removes_variant_when_vram_is_insufficient() {
    let variants = vec![variant("needs-more-vram", "llama.cpp", 8, 8, 1)];
    let filtered = RealModelResolver::filter_by_hardware(&variants, &hardware(6));

    assert!(filtered.is_empty());
}

#[test]
fn hardware_filter_removes_variant_when_ram_is_insufficient() {
    let variants = vec![variant("needs-more-ram", "llama.cpp", 16, 1, 1)];
    let filtered = RealModelResolver::filter_by_hardware(&variants, &hardware(8));

    assert!(filtered.is_empty());
}

#[test]
fn runtime_filter_keeps_only_supported_backends() {
    let variants = vec![
        variant("llama", "llama.cpp", 8, 1, 1),
        variant("vllm", "vllm", 8, 1, 1),
    ];
    let filtered = RealModelResolver::filter_by_runtime(&variants, &["llama.cpp".into()]);

    assert_eq!(
        filtered
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        ["llama"]
    );
}

#[tokio::test]
async fn resolve_selects_the_first_variant_that_fits_the_real_request() {
    let resolver = RealModelResolver::new(vec![manifest_with_backends(
        "selected-model",
        &["llama.cpp"],
        vec![
            variant("needs-more-ram", "llama.cpp", 9, 0, 1),
            variant("needs-more-vram", "llama.cpp", 8, 9, 1),
            variant("compatible", "llama.cpp", 8, 4, 100),
            variant("later-match", "llama.cpp", 0, 0, 1),
        ],
    )])
    .unwrap();

    let outcome = resolver
        .resolve(resolution_request(" selected-model ", hardware(8)))
        .await
        .unwrap();

    match outcome {
        ModelResolutionOutcome::Local(resolved) => {
            assert_eq!(resolved.model, "selected-model");
            assert!(resolved.artifact_source.ends_with("compatible.gguf"));
            assert_eq!(
                resolved.artifact_digest.as_deref(),
                Some("sha256:compatible")
            );
            assert_eq!(resolved.runtime, "llama.cpp");
            assert_eq!(resolved.runtime_version, None);
        }
        ModelResolutionOutcome::Unsupported(reason) => {
            panic!("expected a compatible variant, got {reason:?}");
        }
    }
}

#[tokio::test]
async fn resolve_reports_unsupported_when_no_variant_fits() {
    let resolver = RealModelResolver::new(vec![manifest_with_backends(
        "selected",
        &["llama.cpp"],
        vec![variant("needs-disk", "llama.cpp", 8, 4, 101)],
    )])
    .unwrap();

    let outcome = resolver
        .resolve(resolution_request("selected", hardware(4)))
        .await
        .unwrap();

    assert!(matches!(
        outcome,
        ModelResolutionOutcome::Unsupported(model)
            if matches!(model.reason, LocalModelUnsupportedReason::NoCompatibleVariant)
    ));
}

#[test]
fn resolve_with_capabilities_returns_all_variants_that_survive_filters() {
    let resolver = RealModelResolver::new(vec![manifest(
        "deepseek-r1",
        vec![
            variant("high-vram", "llama.cpp", 8, 16, 1),
            variant("unsupported-runtime", "vllm", 8, 4, 1),
            variant("compatible-first", "llama.cpp", 8, 4, 1),
            variant("compatible-second", "llama.cpp", 8, 4, 1),
        ],
    )])
    .unwrap();

    let result = resolver
        .resolve_with_capabilities("deepseek-r1", &hardware(8), &["llama.cpp".into()])
        .unwrap();

    match result {
        CandidateResolutionOutcome::Candidates(candidates) => {
            assert_eq!(candidates.len(), 2);
            assert_eq!(candidates[0].model_name, "deepseek-r1");
            assert_eq!(candidates[0].variant_id, "compatible-first");
            assert_eq!(candidates[0].quantization, "q4_k_m");
            assert_eq!(candidates[0].min_ram_bytes, 8);
            assert_eq!(candidates[0].min_vram_bytes, 4);
            assert_eq!(candidates[0].disk_size_bytes, 1);
            assert_eq!(candidates[0].runtime, "llama.cpp");
            assert!(candidates[0]
                .artifact_uri
                .ends_with("compatible-first.gguf"));
            assert_eq!(candidates[0].checksum, "sha256:compatible-first");
            assert_eq!(candidates[1].variant_id, "compatible-second");
        }
        CandidateResolutionOutcome::Unsupported(reason) => {
            panic!("expected compatible candidates, got {reason:?}");
        }
    }
}

#[test]
fn resolve_with_capabilities_filters_by_disk_space() {
    let resolver = RealModelResolver::new(vec![manifest_with_backends(
        "deepseek-r1",
        &["llama.cpp"],
        vec![
            variant("exact-fit", "llama.cpp", 8, 4, 100),
            variant("too-large", "llama.cpp", 8, 4, 101),
        ],
    )])
    .unwrap();

    let result = resolver
        .resolve_with_capabilities("deepseek-r1", &hardware(4), &["llama.cpp".into()])
        .unwrap();

    match result {
        CandidateResolutionOutcome::Candidates(candidates) => {
            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0].variant_id, "exact-fit");
            assert_eq!(candidates[0].disk_size_bytes, 100);
        }
        CandidateResolutionOutcome::Unsupported(reason) => {
            panic!("expected the exact-fit variant, got {reason:?}");
        }
    }
}

#[test]
fn resolve_with_capabilities_rejects_empty_public_inputs() {
    let resolver = RealModelResolver::new(Vec::new()).unwrap();

    let model_result =
        resolver.resolve_with_capabilities("  ", &hardware(8), &["llama.cpp".into()]);
    assert!(matches!(
        model_result,
        Err(ModelResolutionError::ResolutionFailed(message))
            if message == "model name must not be empty"
    ));

    let runtime_result =
        resolver.resolve_with_capabilities("deepseek-r1", &hardware(8), &["  ".into()]);
    assert!(matches!(
        runtime_result,
        Err(ModelResolutionError::ResolutionFailed(message))
            if message == "supported runtime names must not be empty"
    ));
}

#[tokio::test]
async fn trait_candidate_resolution_uses_caller_runtime_capabilities() {
    let resolver = RealModelResolver::new(vec![manifest_with_backends(
        "selected-model",
        &["vllm", "llama.cpp"],
        vec![
            variant("vllm-first", "vllm", 0, 0, 1),
            variant("llama-second", "llama.cpp", 0, 0, 1),
        ],
    )])
    .unwrap();

    let result = resolver
        .resolve_candidates(
            resolution_request("selected-model", hardware(0)),
            &["llama.cpp".into()],
        )
        .await
        .unwrap();

    match result {
        CandidateResolutionOutcome::Candidates(candidates) => {
            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0].variant_id, "llama-second");
        }
        CandidateResolutionOutcome::Unsupported(reason) => {
            panic!("expected a compatible candidate, got {reason:?}");
        }
    }
}
