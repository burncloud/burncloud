use async_trait::async_trait;
use burncloud_node_runtime::{
    FakeArtifactPreparer, FakeHealthProbe, FakeProcessManager, FakeReadinessProbe,
    FakeRuntimeAdapter, FakeRuntimePreparer, HardwareProbe, HardwareProfile, NodeComposition,
    NodeState, PreparedRuntime, RuntimePrepareError, RuntimePreparer, RuntimeRequest,
    SystemHardwareProbe,
};
use burncloud_server::{
    node_orchestrator::{LocalPreparationOutcome, ModelDemand, ModelDemandError, NodeOrchestrator},
    node_production::ProductionNode,
};
use burncloud_supply_model::{
    CandidateResolutionOutcome, LocalModelUnsupportedReason, ModelManifest, ModelResolutionError,
    ModelResolutionOutcome, ModelResolutionRequest, ModelResolver, RealModelResolver,
    ResolvedModel, ResolvedModelCandidate, Variant,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

fn variant_with_backend(id: &str, backend: &str, ram: u64, vram: u64) -> Variant {
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

fn variant(id: &str, ram: u64, vram: u64) -> Variant {
    variant_with_backend(id, "llama.cpp", ram, vram)
}

fn manifest(model: &str, variants: Vec<Variant>) -> ModelManifest {
    manifest_with_backends(model, &["llama.cpp"], variants)
}

fn manifest_with_backends(
    model: &str,
    supported_backends: &[&str],
    variants: Vec<Variant>,
) -> ModelManifest {
    ModelManifest {
        model_name: model.into(),
        description: "真实硬件调用链验证".into(),
        supported_backends: supported_backends
            .iter()
            .map(|backend| (*backend).into())
            .collect(),
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
    )])
    .expect("测试 Manifest 应通过后端声明校验");
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

#[tokio::test]
async fn orchestrator_filters_runtime_before_preparing_the_artifact() {
    let resolver = RealModelResolver::new(vec![manifest_with_backends(
        "selected",
        &["vllm", "llama.cpp"],
        vec![
            variant_with_backend("vllm-first", "vllm", 0, 0),
            variant_with_backend("llama-second", "llama.cpp", 0, 0),
        ],
    )])
    .expect("测试 Manifest 应通过后端声明校验");
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
        .expect("兼容候选应完成本地准备");

    assert_eq!(outcome, LocalPreparationOutcome::Ready);
    let plan = orchestrator
        .workload_plan("selected")
        .expect("模型应生成启动计划");
    assert!(plan.process.args[0].contains("llama-second"));
    assert!(!plan.process.args[0].contains("vllm-first"));
}

#[derive(Debug)]
struct RetryResolver;

fn retry_candidate(id: &str) -> ResolvedModelCandidate {
    ResolvedModelCandidate {
        model_name: "selected".into(),
        variant_id: id.into(),
        quantization: "q4_k_m".into(),
        min_ram_bytes: 0,
        min_vram_bytes: 0,
        disk_size_bytes: 1,
        runtime: "llama.cpp".into(),
        artifact_uri: format!("https://example.invalid/{id}.gguf"),
        checksum: format!("sha256:{}", "a".repeat(64)),
    }
}

#[async_trait]
impl ModelResolver for RetryResolver {
    async fn resolve(
        &self,
        request: ModelResolutionRequest,
    ) -> Result<ModelResolutionOutcome, ModelResolutionError> {
        Ok(ModelResolutionOutcome::Local(ResolvedModel {
            model: request.model,
            artifact_source: "https://example.invalid/first.gguf".into(),
            artifact_digest: Some(format!("sha256:{}", "a".repeat(64))),
            runtime: "llama.cpp".into(),
            runtime_version: None,
        }))
    }

    async fn resolve_candidates(
        &self,
        _request: ModelResolutionRequest,
        _supported_runtimes: &[String],
    ) -> Result<CandidateResolutionOutcome, ModelResolutionError> {
        Ok(CandidateResolutionOutcome::Candidates(vec![
            retry_candidate("first"),
            retry_candidate("second"),
        ]))
    }
}

#[derive(Debug, Clone)]
struct FailOnceRuntimePreparer {
    attempts: Arc<AtomicUsize>,
}

#[async_trait]
impl RuntimePreparer for FailOnceRuntimePreparer {
    fn supported_runtimes(&self) -> Vec<String> {
        vec!["llama.cpp".into()]
    }

    async fn prepare(
        &self,
        request: RuntimeRequest,
    ) -> Result<PreparedRuntime, RuntimePrepareError> {
        if self.attempts.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err(RuntimePrepareError::PrepareFailed(
                "first candidate is unavailable".into(),
            ));
        }
        Ok(PreparedRuntime {
            executable: format!("/fake/runtime/{}/server", request.runtime),
        })
    }
}

#[tokio::test]
async fn orchestrator_retries_the_next_candidate_after_runtime_failure() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let machine = NodeComposition::start(
        SystemHardwareProbe::new(),
        FakeArtifactPreparer,
        FailOnceRuntimePreparer {
            attempts: attempts.clone(),
        },
        FakeProcessManager::default(),
        FakeReadinessProbe,
        FakeHealthProbe,
    );
    let mut orchestrator = NodeOrchestrator::new(RetryResolver, FakeRuntimeAdapter, machine);

    let outcome = orchestrator
        .prepare_until_ready(ModelDemand::new("selected").expect("模型名有效"))
        .await
        .expect("第二个候选应完成本地准备");

    assert_eq!(outcome, LocalPreparationOutcome::Ready);
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    let plan = orchestrator
        .workload_plan("selected")
        .expect("第二个候选应生成启动计划");
    assert!(plan.process.args[0].contains("second"));
    assert!(!plan.process.args[0].contains("first"));
}
