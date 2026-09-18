use burncloud_node_runtime::{
    ArtifactPreparer, ArtifactRequest, HardwareProbe, HealthProbe, NodeComposition,
    PreparedArtifact, PreparedRuntime, ProcessHandle, ProcessManager, ProcessPlan, ReadinessProbe,
    ReadinessTarget, ReconcileAction, ReconcileEvidence, RuntimeAdapter, RuntimePreparer,
    RuntimeRequest,
};
use burncloud_service_models::{
    LocalModelUnsupported, ModelResolutionOutcome, ModelResolutionRequest, ModelResolver,
    ResolvedModel,
};
use std::fmt;

use crate::node_request::NodeRequestState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDemand {
    pub model: String,
}
impl ModelDemand {
    pub fn new(model: impl Into<String>) -> Result<Self, ModelDemandError> {
        let model = model.into();
        let model = model.trim();
        if model.is_empty() {
            return Err(ModelDemandError::EmptyModel);
        }
        Ok(Self {
            model: model.into(),
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelDemandError {
    EmptyModel,
}
impl fmt::Display for ModelDemandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("model demand must name a model")
    }
}
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
    resolved: Option<ResolvedModel>,
    artifact: Option<PreparedArtifact>,
    runtime: Option<PreparedRuntime>,
    process: Option<ProcessHandle>,
    readiness: Option<ReadinessTarget>,
}

#[derive(Debug)]
pub struct NodeOrchestrator<M, T, H, A, R, P, Q, E> {
    resolver: M,
    runtime_adapter: T,
    machine: NodeComposition<H, A, R, P, Q, E>,
    active_plan: Option<ProcessPlan>,
    active_model: Option<String>,
    request_state: NodeRequestState,
}

impl<M, T, H, A, R, P, Q, E> NodeOrchestrator<M, T, H, A, R, P, Q, E>
where
    M: ModelResolver,
    T: RuntimeAdapter,
    H: HardwareProbe,
    A: ArtifactPreparer,
    R: RuntimePreparer,
    P: ProcessManager,
    Q: ReadinessProbe,
    E: HealthProbe,
{
    pub fn new(
        resolver: M,
        runtime_adapter: T,
        machine: NodeComposition<H, A, R, P, Q, E>,
    ) -> Self {
        Self {
            resolver,
            runtime_adapter,
            machine,
            active_plan: None,
            active_model: None,
            request_state: NodeRequestState::default(),
        }
    }

    pub fn with_request_state(mut self, request_state: NodeRequestState) -> Self {
        self.request_state = request_state;
        self
    }

    fn publish_state(&self, model: &str) {
        self.request_state
            .publish(model, self.machine.reconciler().state());
    }

    fn observe(&mut self, model: &str, evidence: ReconcileEvidence) -> anyhow::Result<()> {
        self.machine.reconciler_mut().observe(evidence)?;
        self.publish_state(model);
        Ok(())
    }

    fn mark_failed(&mut self, model: &str) -> anyhow::Result<()> {
        self.observe(model, ReconcileEvidence::Failed)
    }
    pub const fn resolver(&self) -> &M {
        &self.resolver
    }
    pub const fn machine(&self) -> &NodeComposition<H, A, R, P, Q, E> {
        &self.machine
    }
    pub fn machine_mut(&mut self) -> &mut NodeComposition<H, A, R, P, Q, E> {
        &mut self.machine
    }
    pub const fn active_plan(&self) -> Option<&ProcessPlan> {
        self.active_plan.as_ref()
    }

    /// Prepare an optional local supply. If Supply says the machine cannot run
    /// the model, stop immediately: no artifact, runtime, process or route work
    /// is performed. The existing request/router path is deliberately untouched.
    pub async fn prepare_until_ready(
        &mut self,
        demand: ModelDemand,
    ) -> anyhow::Result<LocalPreparationOutcome> {
        let mut work = DemandWork::default();
        self.active_model = Some(demand.model.clone());
        loop {
            let action = self.machine.reconciler_mut().next_action()?;
            self.publish_state(&demand.model);
            match action {
                ReconcileAction::Resolve => {
                    let hardware = match self.machine.hardware().inspect().await {
                        Ok(hardware) => hardware,
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    };
                    let accelerator_memory_bytes = hardware
                        .accelerators
                        .iter()
                        .filter_map(|a| a.memory_bytes)
                        .max();
                    match self
                        .resolver
                        .resolve(ModelResolutionRequest {
                            model: demand.model.clone(),
                            accelerator_memory_bytes,
                        })
                        .await
                    {
                        Ok(ModelResolutionOutcome::Local(resolved)) => {
                            work.resolved = Some(resolved);
                            self.observe(&demand.model, ReconcileEvidence::Resolved)?;
                        }
                        Ok(ModelResolutionOutcome::Unsupported(reason)) => {
                            self.observe(&demand.model, ReconcileEvidence::LocalUnsupported)?;
                            return Ok(LocalPreparationOutcome::Unsupported(reason));
                        }
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    }
                }
                ReconcileAction::PrepareArtifact => {
                    let resolved = work
                        .resolved
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("resolved model receipt missing"))?;
                    let artifact = match self
                        .machine
                        .artifacts()
                        .prepare(ArtifactRequest {
                            source: resolved.artifact_source.clone(),
                            expected_digest: resolved.artifact_digest.clone(),
                        })
                        .await
                    {
                        Ok(artifact) => artifact,
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    };
                    if !artifact.verified {
                        self.mark_failed(&demand.model)?;
                        anyhow::bail!("artifact preparer returned unverified artifact");
                    }
                    work.artifact = Some(artifact);
                    self.observe(&demand.model, ReconcileEvidence::ArtifactPrepared)?;
                }
                ReconcileAction::PrepareRuntime => {
                    let resolved = work
                        .resolved
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("resolved model receipt missing"))?;
                    let runtime = match self
                        .machine
                        .runtimes()
                        .prepare(RuntimeRequest {
                            runtime: resolved.runtime.clone(),
                            version: resolved.runtime_version.clone(),
                        })
                        .await
                    {
                        Ok(runtime) => runtime,
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    };
                    work.runtime = Some(runtime);
                    self.observe(&demand.model, ReconcileEvidence::RuntimePrepared)?;
                }
                ReconcileAction::StartProcess => {
                    let runtime = work
                        .runtime
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("runtime receipt missing"))?;
                    let artifact = work
                        .artifact
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("artifact receipt missing"))?;
                    let plan = match self.runtime_adapter.plan(runtime, artifact).await {
                        Ok(plan) => plan,
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    };
                    let handle = match self.machine.processes().start(plan.process.clone()).await {
                        Ok(handle) => handle,
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    };
                    work.process = Some(handle);
                    work.readiness = Some(plan.readiness.clone());
                    self.active_plan = Some(plan);
                    self.observe(&demand.model, ReconcileEvidence::ProcessStarted)?;
                }
                ReconcileAction::VerifyReadiness => {
                    let target = work
                        .readiness
                        .clone()
                        .ok_or_else(|| anyhow::anyhow!("readiness target receipt missing"))?;
                    if let Err(error) = self.machine.readiness().wait_ready(target.clone()).await {
                        self.mark_failed(&demand.model)?;
                        return Err(error.into());
                    }
                    match self.machine.health().is_healthy(target).await {
                        Ok(true) => {}
                        Ok(false) => {
                            self.mark_failed(&demand.model)?;
                            anyhow::bail!("runtime is ready but unhealthy");
                        }
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    }
                    self.observe(&demand.model, ReconcileEvidence::ReadinessVerified)?;
                }
                ReconcileAction::AttachToExistingRouter | ReconcileAction::Noop => {
                    return Ok(LocalPreparationOutcome::Ready)
                }
                ReconcileAction::Recover => {
                    anyhow::bail!("route must be detached before recovery begins")
                }
            }
        }
    }

    pub async fn recover_until_ready(&mut self) -> anyhow::Result<()> {
        let model = self
            .active_model
            .clone()
            .ok_or_else(|| anyhow::anyhow!("active model missing"))?;
        let plan = self
            .active_plan
            .clone()
            .ok_or_else(|| anyhow::anyhow!("active process plan missing"))?;
        loop {
            let action = self.machine.reconciler_mut().next_action()?;
            self.publish_state(&model);
            match action {
                ReconcileAction::StartProcess => {
                    if let Err(error) = self.machine.processes().start(plan.process.clone()).await {
                        self.mark_failed(&model)?;
                        return Err(error.into());
                    }
                    self.observe(&model, ReconcileEvidence::ProcessStarted)?;
                }
                ReconcileAction::VerifyReadiness => {
                    let target = plan.readiness.clone();
                    if let Err(error) = self.machine.readiness().wait_ready(target.clone()).await {
                        self.mark_failed(&model)?;
                        return Err(error.into());
                    }
                    match self.machine.health().is_healthy(target).await {
                        Ok(true) => {}
                        Ok(false) => {
                            self.mark_failed(&model)?;
                            anyhow::bail!("recovered runtime is unhealthy");
                        }
                        Err(error) => {
                            self.mark_failed(&model)?;
                            return Err(error.into());
                        }
                    }
                    self.observe(&model, ReconcileEvidence::ReadinessVerified)?;
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
    use burncloud_node_runtime::{
        FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe, FakeProcessManager,
        FakeReadinessProbe, FakeRuntimeAdapter, FakeRuntimePreparer, NodeState,
    };
    use burncloud_service_models::{
        FakeModelResolver, LocalModelUnsupportedReason, ModelResolutionError,
    };

    fn fake_orchestrator() -> NodeOrchestrator<
        FakeModelResolver,
        FakeRuntimeAdapter,
        FakeHardwareProbe,
        FakeArtifactPreparer,
        FakeRuntimePreparer,
        FakeProcessManager,
        FakeReadinessProbe,
        FakeHealthProbe,
    > {
        NodeOrchestrator::new(
            FakeModelResolver,
            FakeRuntimeAdapter,
            NodeComposition::start(
                FakeHardwareProbe::default(),
                FakeArtifactPreparer,
                FakeRuntimePreparer,
                FakeProcessManager::default(),
                FakeReadinessProbe,
                FakeHealthProbe,
            ),
        )
    }

    #[derive(Debug)]
    struct UnsupportedResolver;

    #[derive(Debug)]
    struct FailingResolver;
    #[async_trait]
    impl ModelResolver for FailingResolver {
        async fn resolve(
            &self,
            _request: ModelResolutionRequest,
        ) -> Result<ModelResolutionOutcome, ModelResolutionError> {
            Err(ModelResolutionError::ResolutionFailed(
                "resolver offline".into(),
            ))
        }
    }
    #[async_trait]
    impl ModelResolver for UnsupportedResolver {
        async fn resolve(
            &self,
            request: ModelResolutionRequest,
        ) -> Result<ModelResolutionOutcome, ModelResolutionError> {
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
    async fn failed_resolution_converges_to_failed_and_stops_reporting_preparing() {
        let request_state = NodeRequestState::default();
        let machine = NodeComposition::start(
            FakeHardwareProbe::default(),
            FakeArtifactPreparer,
            FakeRuntimePreparer,
            FakeProcessManager::default(),
            FakeReadinessProbe,
            FakeHealthProbe,
        );
        let mut orchestrator = NodeOrchestrator::new(FailingResolver, FakeRuntimeAdapter, machine)
            .with_request_state(request_state.clone());

        assert!(orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .is_err());
        assert_eq!(
            orchestrator.machine().reconciler().state(),
            NodeState::Failed
        );
        assert!(request_state.route_miss_response("qwen-4b").is_none());
    }

    #[tokio::test]
    async fn supported_demand_reaches_ready() {
        let mut orchestrator = fake_orchestrator();
        assert_eq!(
            orchestrator
                .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
                .await
                .unwrap(),
            LocalPreparationOutcome::Ready
        );
        assert_eq!(
            orchestrator.machine().reconciler().state(),
            NodeState::Ready
        );
        assert_eq!(
            orchestrator.active_plan().unwrap().local_endpoint,
            "http://127.0.0.1:39122"
        );
    }

    #[tokio::test]
    async fn unsupported_local_model_stops_before_any_preparation_or_route_state() {
        let machine = NodeComposition::start(
            FakeHardwareProbe::default(),
            FakeArtifactPreparer,
            FakeRuntimePreparer,
            FakeProcessManager::default(),
            FakeReadinessProbe,
            FakeHealthProbe,
        );
        let mut orchestrator =
            NodeOrchestrator::new(UnsupportedResolver, FakeRuntimeAdapter, machine);
        let outcome = orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();
        assert!(matches!(outcome, LocalPreparationOutcome::Unsupported(_)));
        assert_eq!(
            orchestrator.machine().reconciler().state(),
            NodeState::LocalUnsupported
        );
        assert_eq!(
            orchestrator
                .machine_mut()
                .reconciler_mut()
                .next_action()
                .unwrap(),
            ReconcileAction::Noop
        );
        assert!(!orchestrator.machine().reconciler().state().is_serving());
        assert!(!orchestrator.machine().reconciler().state().is_failure());
        assert!(orchestrator.active_plan().is_none());
    }
}
