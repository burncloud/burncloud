use burncloud_node_runtime::{
    FakeArtifactPreparer, FakeHealthProbe, FakeProcessManager, FakeReadinessProbe,
    FakeRuntimeAdapter, FakeRuntimePreparer, HardwareProbe, HardwareProfile, NodeComposition,
    NodeState, SystemHardwareProbe,
};
use burncloud_server::{
    node_orchestrator::{LocalPreparationOutcome, ModelDemand, ModelDemandError, NodeOrchestrator},
    node_production::ProductionNode,
};
use burncloud_supply_model::{
    LocalModelUnsupportedReason, ModelManifest, ModelResolutionOutcome, RealModelResolver, Variant,
};

fn variant(id: &str, ram: u64, vram: u64) -> Variant {
    Variant {
        id: id.into(),
        model_size: None,
        quantization: "q4_k_m".into(),
        backend: "llama.cpp".into(),
        min_ram_bytes: ram,
        min_vram_bytes: vram,
        disk_size_bytes: 1,
        checksum: format!("sha256:{}", "a".repeat(64)),
        artifact_uri: format!("https://example.invalid/{id}.gguf"),
    }
}

fn manifest(model: &str, variants: Vec<Variant>) -> ModelManifest {
    ModelManifest {
        model_name: model.into(),
        description: "真实硬件调用链验证".into(),
        supported_backends: vec!["llama.cpp".into()],
        version: "1".into(),
        variants,
    }
}

fn maximum_vram(profile: &HardwareProfile) -> u64 {
    profile
        .accelerators
        .iter()
        .filter_map(|gpu| gpu.memory_bytes)
        .max()
        .unwrap_or(0)
}

#[test]
fn production_node_can_be_assembled_before_starting_an_async_runtime() {
    let _node = ProductionNode::new();
    let _default_node = ProductionNode::default();
}

#[tokio::test]
async fn production_entry_rejects_an_empty_model() {
    let error = ProductionNode::new()
        .resolve_selected_model("  ", Vec::new())
        .await
        .expect_err("空模型名应在探测前被拒绝");
    assert_eq!(
        error.downcast_ref::<ModelDemandError>(),
        Some(&ModelDemandError::EmptyModel)
    );
}

#[tokio::test]
async fn production_entry_uses_real_resources_to_choose_a_compatible_variant() {
    let profile = SystemHardwareProbe::new()
        .inspect()
        .await
        .expect("真实探测应成功");
    let vram = maximum_vram(&profile);
    println!(
        "实测硬件: {}",
        serde_json::to_string(&profile).expect("硬件快照可序列化")
    );
    let outcome = ProductionNode::new()
        .resolve_selected_model(
            " selected ",
            vec![
                manifest("other", vec![variant("wrong-model", 0, 0)]),
                manifest(
                    "selected",
                    vec![
                        variant("ram-exceeded", profile.memory_bytes + 1, 0),
                        variant("vram-exceeded", 0, vram + 1),
                        variant("compatible", profile.memory_bytes, vram),
                    ],
                ),
            ],
        )
        .await
        .expect("生产入口应返回判断结果");
    println!("生产入口判断结果: {outcome:?}");
    let ModelResolutionOutcome::Local(resolved) = outcome else {
        panic!("应选中符合真实资源的变体");
    };
    assert_eq!(resolved.model, "selected");
    assert_eq!(
        resolved.artifact_source,
        "https://example.invalid/compatible.gguf"
    );
}

#[tokio::test]
async fn production_entry_rejects_model_requirements_above_real_ram_or_vram() {
    let profile = SystemHardwareProbe::new()
        .inspect()
        .await
        .expect("真实探测应成功");
    let node = ProductionNode::new();
    for candidate in [
        variant("ram-exceeded", profile.memory_bytes + 1, 0),
        variant("vram-exceeded", 0, maximum_vram(&profile) + 1),
    ] {
        let outcome = node
            .resolve_selected_model("selected", vec![manifest("selected", vec![candidate])])
            .await
            .expect("不兼容应作为正常判断结果返回");
        assert!(
            matches!(outcome, ModelResolutionOutcome::Unsupported(reason)
            if reason.reason == LocalModelUnsupportedReason::NoCompatibleVariant)
        );
    }
}

#[tokio::test]
async fn production_entry_propagates_catalog_errors() {
    let result = ProductionNode::new()
        .resolve_selected_model("missing", Vec::new())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn orchestrator_passes_real_ram_to_the_resolver_and_stops_before_preparation() {
    let profile = SystemHardwareProbe::new()
        .inspect()
        .await
        .expect("真实探测应成功");
    let resolver = RealModelResolver::new(vec![manifest(
        "selected",
        vec![variant("too-large", profile.memory_bytes + 1, 0)],
    )]);
    let machine = NodeComposition::start(
        SystemHardwareProbe::new(),
        FakeArtifactPreparer,
        FakeRuntimePreparer,
        FakeProcessManager::default(),
        FakeReadinessProbe,
        FakeHealthProbe,
    );
    let mut orchestrator = NodeOrchestrator::new(resolver, FakeRuntimeAdapter, machine);
    let outcome = orchestrator
        .prepare_until_ready(ModelDemand::new("selected").expect("模型名有效"))
        .await
        .expect("不兼容应正常返回");
    assert!(matches!(outcome, LocalPreparationOutcome::Unsupported(_)));
    assert_eq!(
        orchestrator.workload_state("selected"),
        Some(NodeState::LocalUnsupported)
    );
    assert!(orchestrator.workload_plan("selected").is_none());
    assert!(orchestrator.workload_attachment_id("selected").is_none());
}
