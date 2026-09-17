use burncloud_node_runtime::{
    ArtifactPreparer, ArtifactRequest, HardwareProbe, HealthProbe, NodeComposition, ProcessHandle,
    ProcessManager, ProcessSpec, ReadinessProbe, ReadinessTarget, ReconcileAction,
    ReconcileEvidence, RuntimePreparer, RuntimeRequest,
};
use burncloud_service_models::{
    LocalModelUnsupported, ModelResolutionOutcome, ModelResolutionRequest, ModelResolver, ResolvedModel,
};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDemand { pub model: String }
impl ModelDemand {
    pub fn new(model: impl Into<String>) -> Result<Self, ModelDemandError> {
        let model = model.into(); let model = model.trim();
        if model.is_empty() { return Err(ModelDemandError::EmptyModel); }
        Ok(Self { model: model.into() })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelDemandError { EmptyModel }
impl fmt::Display for ModelDemandError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("model demand must name a model") } }
impl std::error::Error for ModelDemandError {}

/// Result of the optional local-supply preparation rail.
/// Unsupported means "do nothing locally"; it must never become a routing error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalPreparationOutcome {
    Ready,
    Unsupported(LocalModelUnsupported),
}

#[derive(Debug, Default)]
struct DemandWork {
    resolved: Option<ResolvedModel>, artifact_path: Option<String>, runtime_executable: Option<String>,
    process: Option<ProcessHandle>, readiness: Option<ReadinessTarget>,
}

#[derive(Debug)]
pub struct NodeOrchestrator<M, H, A, R, P, Q, E> { resolver: M, machine: NodeComposition<H, A, R, P, Q, E> }

impl<M, H, A, R, P, Q, E> NodeOrchestrator<M, H, A, R, P, Q, E>
where M: ModelResolver, H: HardwareProbe, A: ArtifactPreparer, R: RuntimePreparer,
      P: ProcessManager, Q: ReadinessProbe, E: HealthProbe,
{
    pub const fn new(resolver: M, machine: NodeComposition<H, A, R, P, Q, E>) -> Self { Self { resolver, machine } }
    pub const fn resolver(&self) -> &M { &self.resolver }
    pub const fn machine(&self) -> &NodeComposition<H, A, R, P, Q, E> { &self.machine }
    pub fn machine_mut(&mut self) -> &mut NodeComposition<H, A, R, P, Q, E> { &mut self.machine }

    /// Prepare an optional local supply. If Supply says the machine cannot run
    /// the model, stop immediately: no artifact, runtime, process or route work
    /// is performed. The existing request/router path is deliberately untouched.
    pub async fn prepare_until_ready(&mut self, demand: ModelDemand) -> anyhow::Result<LocalPreparationOutcome> {
        let mut work = DemandWork::default();
        loop {
            match self.machine.reconciler_mut().next_action()? {
                ReconcileAction::Resolve => {
                    let hardware = self.machine.hardware().inspect().await?;
                    let accelerator_memory_bytes = hardware.accelerators.iter().filter_map(|a| a.memory_bytes).max();
                    match self.resolver.resolve(ModelResolutionRequest { model: demand.model.clone(), accelerator_memory_bytes }).await? {
                        ModelResolutionOutcome::Local(resolved) => {
                            work.resolved = Some(resolved);
                            self.machine.reconciler_mut().observe(ReconcileEvidence::Resolved)?;
                        }
                        ModelResolutionOutcome::Unsupported(reason) => {
                            return Ok(LocalPreparationOutcome::Unsupported(reason));
                        }
                    }
                }
                ReconcileAction::PrepareArtifact => {
                    let resolved = work.resolved.as_ref().ok_or_else(|| anyhow::anyhow!("resolved model receipt missing"))?;
                    let artifact = self.machine.artifacts().prepare(ArtifactRequest { source: resolved.artifact_source.clone(), expected_digest: resolved.artifact_digest.clone() }).await?;
                    if !artifact.verified { anyhow::bail!("artifact preparer returned unverified artifact"); }
                    work.artifact_path = Some(artifact.local_path);
                    self.machine.reconciler_mut().observe(ReconcileEvidence::ArtifactPrepared)?;
                }
                ReconcileAction::PrepareRuntime => {
                    let resolved = work.resolved.as_ref().ok_or_else(|| anyhow::anyhow!("resolved model receipt missing"))?;
                    let runtime = self.machine.runtimes().prepare(RuntimeRequest { runtime: resolved.runtime.clone(), version: resolved.runtime_version.clone() }).await?;
                    work.runtime_executable = Some(runtime.executable);
                    self.machine.reconciler_mut().observe(ReconcileEvidence::RuntimePrepared)?;
                }
                ReconcileAction::StartProcess => {
                    let program = work.runtime_executable.clone().ok_or_else(|| anyhow::anyhow!("runtime receipt missing"))?;
                    let artifact = work.artifact_path.clone().ok_or_else(|| anyhow::anyhow!("artifact receipt missing"))?;
                    let handle = self.machine.processes().start(ProcessSpec { program, args: vec![artifact] }).await?;
                    work.process = Some(handle);
                    work.readiness = Some(ReadinessTarget { endpoint: "http://127.0.0.1:39122/health".into() });
                    self.machine.reconciler_mut().observe(ReconcileEvidence::ProcessStarted)?;
                }
                ReconcileAction::VerifyReadiness => {
                    let target = work.readiness.clone().ok_or_else(|| anyhow::anyhow!("readiness target receipt missing"))?;
                    self.machine.readiness().wait_ready(target.clone()).await?;
                    if !self.machine.health().is_healthy(target).await? { anyhow::bail!("runtime is ready but unhealthy"); }
                    self.machine.reconciler_mut().observe(ReconcileEvidence::ReadinessVerified)?;
                }
                ReconcileAction::AttachToExistingRouter | ReconcileAction::Noop => return Ok(LocalPreparationOutcome::Ready),
                ReconcileAction::Recover => anyhow::bail!("route must be detached before recovery begins"),
            }
        }
    }

    pub async fn recover_until_ready(&mut self) -> anyhow::Result<()> {
        let mut readiness = None;
        loop {
            match self.machine.reconciler_mut().next_action()? {
                ReconcileAction::StartProcess => {
                    self.machine.processes().start(ProcessSpec { program: "recovery-runtime".into(), args: Vec::new() }).await?;
                    readiness = Some(ReadinessTarget { endpoint: "http://127.0.0.1:39122/health".into() });
                    self.machine.reconciler_mut().observe(ReconcileEvidence::ProcessStarted)?;
                }
                ReconcileAction::VerifyReadiness => {
                    let target = readiness.clone().ok_or_else(|| anyhow::anyhow!("recovery readiness target receipt missing"))?;
                    self.machine.readiness().wait_ready(target.clone()).await?;
                    if !self.machine.health().is_healthy(target).await? { anyhow::bail!("recovered runtime is unhealthy"); }
                    self.machine.reconciler_mut().observe(ReconcileEvidence::ReadinessVerified)?;
                }
                ReconcileAction::AttachToExistingRouter => return Ok(()),
                other => anyhow::bail!("unexpected recovery action: {other:?}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use burncloud_node_runtime::{FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe, FakeProcessManager, FakeReadinessProbe, FakeRuntimePreparer, NodeState};
    use burncloud_service_models::{FakeModelResolver, LocalModelUnsupportedReason, ModelResolutionError};

    fn fake_orchestrator() -> NodeOrchestrator<FakeModelResolver, FakeHardwareProbe, FakeArtifactPreparer, FakeRuntimePreparer, FakeProcessManager, FakeReadinessProbe, FakeHealthProbe> {
        NodeOrchestrator::new(FakeModelResolver, NodeComposition::start(FakeHardwareProbe::default(), FakeArtifactPreparer, FakeRuntimePreparer, FakeProcessManager::default(), FakeReadinessProbe, FakeHealthProbe))
    }

    #[derive(Debug)] struct UnsupportedResolver;
    #[async_trait]
    impl ModelResolver for UnsupportedResolver {
        async fn resolve(&self, request: ModelResolutionRequest) -> Result<ModelResolutionOutcome, ModelResolutionError> {
            Ok(ModelResolutionOutcome::Unsupported(LocalModelUnsupported {
                model: request.model,
                reason: LocalModelUnsupportedReason::NoCompatibleVariant,
            }))
        }
    }

    #[test]
    fn demand_contains_only_the_requested_model() {
        assert_eq!(ModelDemand::new(" qwen-4b ").unwrap().model, "qwen-4b");
        assert_eq!(ModelDemand::new("   "), Err(ModelDemandError::EmptyModel));
    }

    #[tokio::test]
    async fn supported_demand_reaches_ready() {
        let mut orchestrator = fake_orchestrator();
        assert_eq!(orchestrator.prepare_until_ready(ModelDemand::new("qwen-4b").unwrap()).await.unwrap(), LocalPreparationOutcome::Ready);
        assert_eq!(orchestrator.machine().reconciler().state(), NodeState::Ready);
    }

    #[tokio::test]
    async fn unsupported_local_model_stops_before_any_preparation_or_route_state() {
        let machine = NodeComposition::start(FakeHardwareProbe::default(), FakeArtifactPreparer, FakeRuntimePreparer, FakeProcessManager::default(), FakeReadinessProbe, FakeHealthProbe);
        let mut orchestrator = NodeOrchestrator::new(UnsupportedResolver, machine);
        let outcome = orchestrator.prepare_until_ready(ModelDemand::new("qwen-4b").unwrap()).await.unwrap();
        assert!(matches!(outcome, LocalPreparationOutcome::Unsupported(_)));
        assert_eq!(orchestrator.machine().reconciler().state(), NodeState::Resolving);
        assert!(!orchestrator.machine().reconciler().state().is_serving());
    }
}
