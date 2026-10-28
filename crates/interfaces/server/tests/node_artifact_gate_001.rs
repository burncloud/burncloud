use async_trait::async_trait;
use burncloud_node_runtime::{
    ArtifactPreparation, ArtifactPrepareError, ArtifactVerificationStatus, Digest,
    FakeHardwareProbe, FakeHealthProbe, FakeProcessManager, FakeReadinessProbe, FakeRuntimeAdapter,
    FakeRuntimePreparer, NodeComposition, PreparedArtifact,
};
use burncloud_server::node_orchestrator::{ModelDemand, NodeOrchestrator};
use burncloud_supply_model::{
    ModelResolutionOutcome, ModelResolutionRequest, ModelResolver, ResolvedModel,
};
use reqwest::Url;

fn closed_monitor_errors(
) -> tokio::sync::mpsc::UnboundedReceiver<burncloud_node_runtime::deployment::DeploymentMonitorError>
{
    let (_, receiver) = tokio::sync::mpsc::unbounded_channel();
    receiver
}

struct FixedResolver;

#[async_trait]
impl ModelResolver for FixedResolver {
    async fn resolve(
        &self,
        request: ModelResolutionRequest,
    ) -> Result<ModelResolutionOutcome, burncloud_supply_model::ModelResolutionError> {
        Ok(ModelResolutionOutcome::Local(ResolvedModel {
            model: request.model,
            artifact_source: "https://modelscope.cn/models/example/resolve/main/model.safetensors"
                .into(),
            artifact_digest: Some(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
            ),
            runtime: "llama.cpp".into(),
            runtime_version: Some("test".into()),
        }))
    }
}

struct UnverifiedArtifactPreparer;

#[async_trait]
impl ArtifactPreparation for UnverifiedArtifactPreparer {
    async fn prepare(
        &self,
        _url: Url,
        _expected_digest: Digest,
    ) -> Result<PreparedArtifact, ArtifactPrepareError> {
        PreparedArtifact::new("", ArtifactVerificationStatus::Failed)
            .map_err(|error| ArtifactPrepareError::PrepareFailed(error.to_string()))
    }
}

#[tokio::test]
async fn unverified_artifact_receipt_fails_closed_before_runtime_start() {
    let machine = NodeComposition::start(
        FakeHardwareProbe::default(),
        UnverifiedArtifactPreparer,
        FakeRuntimePreparer,
        std::sync::Arc::new(FakeProcessManager::default()),
        std::sync::Arc::new(FakeReadinessProbe),
        std::sync::Arc::new(FakeHealthProbe),
        closed_monitor_errors(),
    );
    let mut orchestrator = NodeOrchestrator::new(FixedResolver, FakeRuntimeAdapter, machine);

    let result = orchestrator
        .prepare_until_ready(ModelDemand::new("qwen-4b").expect("固定模型名应合法"))
        .await;

    assert!(result.is_err());
    assert_eq!(
        orchestrator.workload_state("qwen-4b"),
        Some(burncloud_node_runtime::NodeState::Failed)
    );
    assert!(orchestrator.workload_plan("qwen-4b").is_none());
}
