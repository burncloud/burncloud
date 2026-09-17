use burncloud_node_runtime::{
    ArtifactPreparer, ArtifactRequest, HardwareProbe, HealthProbe, NodeComposition, ProcessHandle,
    ProcessManager, ProcessSpec, ReadinessProbe, ReadinessTarget, ReconcileAction,
    ReconcileEvidence, RuntimePreparer, RuntimeRequest,
};
use burncloud_service_models::{ModelResolutionRequest, ModelResolver, ResolvedModel};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDemand { pub model: String }

impl ModelDemand {
    pub fn new(model: impl Into<String>) -> Result<Self, ModelDemandError> {
        let model = model.into();
        let model = model.trim();
        if model.is_empty() { return Err(ModelDemandError::EmptyModel); }
        Ok(Self { model: model.into() })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelDemandError { EmptyModel }
impl fmt::Display for ModelDemandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("model demand must name a model") }
}
impl std::error::Error for ModelDemandError {}

#[derive(Debug, Default)]
struct DemandWork {
    resolved: Option<ResolvedModel>,
    artifact_path: Option<String>,
    runtime_executable: Option<String>,
    process: Option<ProcessHandle>,
    readiness: Option<ReadinessTarget>,
}

#[derive(Debug)]
pub struct NodeOrchestrator<M, H, A, R, P, Q, E> {
    resolver: M,
    machine: NodeComposition<H, A, R, P, Q, E>,
}

impl<M, H, A, R, P, Q, E> NodeOrchestrator<M, H, A, R, P, Q, E>
where
    M: ModelResolver,
    H: HardwareProbe,
    A: ArtifactPreparer,
    R: RuntimePreparer,
    P: ProcessManager,
    Q: ReadinessProbe,
    E: HealthProbe,
{
    pub const fn new(resolver: M, machine: NodeComposition<H, A, R, P, Q, E>) -> Self { Self { resolver, machine } }
    pub const fn resolver(&self) -> &M { &self.resolver }
    pub const fn machine(&self) -> &NodeComposition<H, A, R, P, Q, E> { &self.machine }
    pub fn machine_mut(&mut self) -> &mut NodeComposition<H, A, R, P, Q, E> { &mut self.machine }

    pub async fn prepare_until_ready(&mut self, demand: ModelDemand) -> anyhow::Result<()> {
        let mut work = DemandWork::default();
        loop {
            let action = self.machine.reconciler_mut().next_action()?;
            match action {
                ReconcileAction::Resolve => {
                    let hardware = self.machine.hardware().inspect().await?;
                    let accelerator_memory_bytes = hardware.accelerators.iter().filter_map(|a| a.memory_bytes).max();
                    let resolved = self.resolver.resolve(ModelResolutionRequest {
                        model: demand.model.clone(), accelerator_memory_bytes,
                    }).await?;
                    work.resolved = Some(resolved);
                    self.machine.reconciler_mut().observe(ReconcileEvidence::Resolved)?;
                }
                ReconcileAction::PrepareArtifact => {
                    let resolved = work.resolved.as_ref().ok_or_else(|| anyhow::anyhow!("resolved model receipt missing"))?;
                    let artifact = self.machine.artifacts().prepare(ArtifactRequest {
                        source: resolved.artifact_source.clone(), expected_digest: resolved.artifact_digest.clone(),
                    }).await?;
                    if !artifact.verified { anyhow::bail!("artifact preparer returned unverified artifact"); }
                    work.artifact_path = Some(artifact.local_path);
                    self.machine.reconciler_mut().observe(ReconcileEvidence::ArtifactPrepared)?;
                }
                ReconcileAction::PrepareRuntime => {
                    let resolved = work.resolved.as_ref().ok_or_else(|| anyhow::anyhow!("resolved model receipt missing"))?;
                    let runtime = self.machine.runtimes().prepare(RuntimeRequest {
                        runtime: resolved.runtime.clone(), version: resolved.runtime_version.clone(),
                    }).await?;
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
                ReconcileAction::AttachToExistingRouter => return Ok(()),
                ReconcileAction::Recover => anyhow::bail!("route must be detached before recovery begins"),
                ReconcileAction::Noop => return Ok(()),
            }
        }
    }

    /// Drive only the machine-owned recovery rail after application wiring has
    /// proved the old Traffic attachment was detached and moved state to Starting.
    pub async fn recover_until_ready(&mut self) -> anyhow::Result<()> {
        let mut readiness = None;
        loop {
            match self.machine.reconciler_mut().next_action()? {
                ReconcileAction::StartProcess => {
                    // Recovery does not resolve/download again. The real process
                    // owner will replace this skeleton ProcessSpec construction.
                    self.machine.processes().start(ProcessSpec {
                        program: "recovery-runtime".into(),
                        args: Vec::new(),
                    }).await?;
                    readiness = Some(ReadinessTarget {
                        endpoint: "http://127.0.0.1:39122/health".into(),
                    });
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
    use burncloud_node_runtime::{FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe, FakeProcessManager, FakeReadinessProbe, FakeRuntimePreparer, NodeState};
    use burncloud_service_models::FakeModelResolver;

    fn fake_orchestrator() -> NodeOrchestrator<FakeModelResolver, FakeHardwareProbe, FakeArtifactPreparer, FakeRuntimePreparer, FakeProcessManager, FakeReadinessProbe, FakeHealthProbe> {
        NodeOrchestrator::new(FakeModelResolver, NodeComposition::start(
            FakeHardwareProbe::default(), FakeArtifactPreparer, FakeRuntimePreparer,
            FakeProcessManager::default(), FakeReadinessProbe, FakeHealthProbe,
        ))
    }

    #[test]
    fn demand_contains_only_the_requested_model() {
        assert_eq!(ModelDemand::new(" qwen-4b ").unwrap().model, "qwen-4b");
        assert_eq!(ModelDemand::new("   "), Err(ModelDemandError::EmptyModel));
    }

    #[tokio::test]
    async fn one_demand_automatically_dispatches_owner_ports_until_ready() {
        let mut orchestrator = fake_orchestrator();
        orchestrator.prepare_until_ready(ModelDemand::new("qwen-4b").unwrap()).await.unwrap();
        assert_eq!(orchestrator.machine().reconciler().state(), NodeState::Ready);
        assert!(!orchestrator.machine().reconciler().state().is_serving());
    }
}
