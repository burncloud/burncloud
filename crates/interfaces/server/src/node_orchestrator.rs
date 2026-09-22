use burncloud_node_runtime::{
    ArtifactPreparer, ArtifactRequest, DemandReconciler, HardwareProbe, HealthProbe,
    NodeComposition, NodeState, PreparedArtifact, PreparedRuntime, ProcessHandle, ProcessManager,
    ProcessPlan, ReadinessProbe, ReconcileAction, ReconcileEvidence, RuntimeAdapter,
    RuntimePreparer, RuntimeRequest,
};
use burncloud_router::local_attachment::LocalRouteAttachmentId;
use burncloud_service_models::{
    LocalModelUnsupported, ModelResolutionOutcome, ModelResolutionRequest, ModelResolver,
    ResolvedModel,
};
use std::collections::HashMap;
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
    Routable(LocalRouteAttachmentId),
    Unsupported(LocalModelUnsupported),
}

/// Application-owned lifecycle for exactly one requested model.
///
/// The machine capabilities are shared, but convergence truth is not: every
/// model owns an independent reconciler, process plan, process receipt and
/// Traffic attachment identity.
#[derive(Debug, Default)]
struct ModelWorkload {
    reconciler: DemandReconciler,
    resolved: Option<ResolvedModel>,
    artifact: Option<PreparedArtifact>,
    runtime: Option<PreparedRuntime>,
    plan: Option<ProcessPlan>,
    process: Option<ProcessHandle>,
    attachment_id: Option<LocalRouteAttachmentId>,
    detached_attachment_id: Option<LocalRouteAttachmentId>,
    unsupported: Option<LocalModelUnsupported>,
}

#[derive(Debug)]
pub struct NodeOrchestrator<M, T, H, A, R, P, Q, E> {
    resolver: M,
    runtime_adapter: T,
    machine: NodeComposition<H, A, R, P, Q, E>,
    workloads: HashMap<String, ModelWorkload>,
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
            workloads: HashMap::new(),
            request_state: NodeRequestState::default(),
        }
    }

    pub fn with_request_state(mut self, request_state: NodeRequestState) -> Self {
        self.request_state = request_state;
        self
    }

    pub fn request_state(&self) -> NodeRequestState {
        self.request_state.clone()
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

    pub fn workload_count(&self) -> usize {
        self.workloads.len()
    }

    pub fn workload_state(&self, model: &str) -> Option<NodeState> {
        self.workloads
            .get(model)
            .map(|workload| workload.reconciler.state())
    }

    pub fn workload_plan(&self, model: &str) -> Option<&ProcessPlan> {
        self.workloads.get(model)?.plan.as_ref()
    }

    pub fn workload_attachment_id(&self, model: &str) -> Option<LocalRouteAttachmentId> {
        self.workloads.get(model)?.attachment_id
    }

    fn ensure_workload(&mut self, model: &str) {
        self.workloads.entry(model.to_string()).or_default();
    }

    fn publish_state(&self, model: &str) {
        if let Some(state) = self.workload_state(model) {
            self.request_state.publish(model, state);
        }
    }

    fn next_action(&mut self, model: &str) -> anyhow::Result<ReconcileAction> {
        self.ensure_workload(model);
        let action = self
            .workloads
            .get_mut(model)
            .expect("workload must exist")
            .reconciler
            .next_action()?;
        self.publish_state(model);
        Ok(action)
    }

    fn observe(&mut self, model: &str, evidence: ReconcileEvidence) -> anyhow::Result<()> {
        self.ensure_workload(model);
        self.workloads
            .get_mut(model)
            .expect("workload must exist")
            .reconciler
            .observe(evidence)?;
        self.publish_state(model);
        Ok(())
    }

    pub(crate) fn mark_failed(&mut self, model: &str) -> anyhow::Result<()> {
        self.observe(model, ReconcileEvidence::Failed)
    }

    pub(crate) fn ensure_ready_for_attachment(&self, model: &str) -> anyhow::Result<()> {
        let workload = self
            .workloads
            .get(model)
            .ok_or_else(|| anyhow::anyhow!("workload missing for model '{model}'"))?;

        if workload.reconciler.state() != NodeState::Ready {
            anyhow::bail!(
                "model '{model}' is not ready for attachment: {:?}",
                workload.reconciler.state()
            );
        }
        if workload.attachment_id.is_some() {
            anyhow::bail!("model '{model}' already owns a route attachment");
        }
        if workload.plan.is_none() {
            anyhow::bail!("model '{model}' is Ready without a process plan");
        }
        Ok(())
    }

    pub(crate) fn record_attachment(
        &mut self,
        model: &str,
        attachment_id: LocalRouteAttachmentId,
    ) -> anyhow::Result<()> {
        self.ensure_ready_for_attachment(model)?;

        // Store the Traffic receipt before publishing Routable. This method is
        // synchronous, so no observer can see a Routable request projection
        // without the workload already owning the exact attachment identity.
        {
            let workload = self.workloads.get_mut(model).expect("workload must exist");
            workload.attachment_id = Some(attachment_id);
            workload.detached_attachment_id = None;
            if let Err(error) = workload
                .reconciler
                .observe(ReconcileEvidence::RouterAttached)
            {
                workload.attachment_id = None;
                return Err(error.into());
            }
        }

        self.publish_state(model);
        Ok(())
    }

    pub(crate) fn mark_unhealthy(&mut self, model: &str) -> anyhow::Result<()> {
        self.observe(model, ReconcileEvidence::BecameUnhealthy)
    }

    pub(crate) fn clear_attachment(
        &mut self,
        model: &str,
        attachment_id: LocalRouteAttachmentId,
    ) -> anyhow::Result<()> {
        let workload = self
            .workloads
            .get_mut(model)
            .ok_or_else(|| anyhow::anyhow!("workload missing for model '{model}'"))?;
        if workload.attachment_id != Some(attachment_id) {
            anyhow::bail!(
                "attachment mismatch for model '{model}': expected {:?}, got {:?}",
                workload.attachment_id,
                attachment_id
            );
        }
        workload.attachment_id = None;
        workload.detached_attachment_id = Some(attachment_id);
        Ok(())
    }

    pub(crate) fn begin_recovery(
        &mut self,
        model: &str,
        detached_attachment_id: LocalRouteAttachmentId,
    ) -> anyhow::Result<()> {
        let workload = self
            .workloads
            .get_mut(model)
            .ok_or_else(|| anyhow::anyhow!("workload missing for model '{model}'"))?;

        if workload.detached_attachment_id != Some(detached_attachment_id) {
            anyhow::bail!(
                "detached route receipt mismatch for model '{model}': expected {:?}, got {:?}",
                workload.detached_attachment_id,
                detached_attachment_id
            );
        }

        workload.reconciler.retry()?;
        workload.detached_attachment_id = None;
        self.publish_state(model);
        Ok(())
    }

    /// Prepare an optional local supply. If Supply says the machine cannot run
    /// the model, stop immediately: no artifact, runtime, process or route work
    /// is performed. The existing request/router path is deliberately untouched.
    pub async fn prepare_until_ready(
        &mut self,
        demand: ModelDemand,
    ) -> anyhow::Result<LocalPreparationOutcome> {
        self.ensure_workload(&demand.model);

        loop {
            let action = self.next_action(&demand.model)?;
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
                            self.workloads
                                .get_mut(&demand.model)
                                .expect("workload must exist")
                                .resolved = Some(resolved);
                            self.observe(&demand.model, ReconcileEvidence::Resolved)?;
                        }
                        Ok(ModelResolutionOutcome::Unsupported(reason)) => {
                            self.workloads
                                .get_mut(&demand.model)
                                .expect("workload must exist")
                                .unsupported = Some(reason.clone());
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
                    let resolved = self
                        .workloads
                        .get(&demand.model)
                        .and_then(|workload| workload.resolved.clone())
                        .ok_or_else(|| anyhow::anyhow!("resolved model receipt missing"))?;
                    let artifact = match self
                        .machine
                        .artifacts()
                        .prepare(ArtifactRequest {
                            source: resolved.artifact_source,
                            expected_digest: resolved.artifact_digest,
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
                    self.workloads
                        .get_mut(&demand.model)
                        .expect("workload must exist")
                        .artifact = Some(artifact);
                    self.observe(&demand.model, ReconcileEvidence::ArtifactPrepared)?;
                }
                ReconcileAction::PrepareRuntime => {
                    let resolved = self
                        .workloads
                        .get(&demand.model)
                        .and_then(|workload| workload.resolved.clone())
                        .ok_or_else(|| anyhow::anyhow!("resolved model receipt missing"))?;
                    let runtime = match self
                        .machine
                        .runtimes()
                        .prepare(RuntimeRequest {
                            runtime: resolved.runtime,
                            version: resolved.runtime_version,
                        })
                        .await
                    {
                        Ok(runtime) => runtime,
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    };
                    self.workloads
                        .get_mut(&demand.model)
                        .expect("workload must exist")
                        .runtime = Some(runtime);
                    self.observe(&demand.model, ReconcileEvidence::RuntimePrepared)?;
                }
                ReconcileAction::StartProcess => {
                    let (runtime, artifact) = self
                        .workloads
                        .get(&demand.model)
                        .map(|workload| (workload.runtime.clone(), workload.artifact.clone()))
                        .ok_or_else(|| {
                            anyhow::anyhow!("workload missing for model '{}'", demand.model)
                        })?;
                    let runtime =
                        runtime.ok_or_else(|| anyhow::anyhow!("runtime receipt missing"))?;
                    let artifact =
                        artifact.ok_or_else(|| anyhow::anyhow!("artifact receipt missing"))?;

                    let plan = match self.runtime_adapter.plan(&runtime, &artifact).await {
                        Ok(plan) => plan,
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    };

                    // Persist the launch receipt before awaiting process start.
                    // If this orchestration call is cancelled while start() is
                    // pending, a later reconcile call still has all framework
                    // evidence needed to continue from Starting.
                    self.workloads
                        .get_mut(&demand.model)
                        .expect("workload must exist")
                        .plan = Some(plan.clone());

                    let handle = match self.machine.processes().start(plan.process.clone()).await {
                        Ok(handle) => handle,
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    };
                    self.workloads
                        .get_mut(&demand.model)
                        .expect("workload must exist")
                        .process = Some(handle);
                    self.observe(&demand.model, ReconcileEvidence::ProcessStarted)?;
                }
                ReconcileAction::VerifyReadiness => {
                    let target = self
                        .workload_plan(&demand.model)
                        .map(|plan| plan.readiness.clone())
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
                ReconcileAction::AttachToExistingRouter => {
                    return Ok(LocalPreparationOutcome::Ready)
                }
                ReconcileAction::Noop => match self.workload_state(&demand.model) {
                    Some(NodeState::Routable) => {
                        let attachment_id =
                            self.workload_attachment_id(&demand.model).ok_or_else(|| {
                                anyhow::anyhow!(
                                    "routable model '{}' has no route attachment receipt",
                                    demand.model
                                )
                            })?;
                        return Ok(LocalPreparationOutcome::Routable(attachment_id));
                    }
                    Some(NodeState::LocalUnsupported) => {
                        let reason = self
                            .workloads
                            .get(&demand.model)
                            .and_then(|workload| workload.unsupported.clone())
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "unsupported model '{}' has no unsupported receipt",
                                    demand.model
                                )
                            })?;
                        return Ok(LocalPreparationOutcome::Unsupported(reason));
                    }
                    state => anyhow::bail!(
                        "unexpected no-op state for model '{}': {state:?}",
                        demand.model
                    ),
                },
                ReconcileAction::Recover => {
                    anyhow::bail!("explicit recovery is required for model '{}'", demand.model)
                }
            }
        }
    }

    pub async fn recover_until_ready(&mut self, model: &str) -> anyhow::Result<()> {
        let plan = self
            .workload_plan(model)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("process plan missing for model '{model}'"))?;

        loop {
            let action = self.next_action(model)?;
            match action {
                ReconcileAction::StartProcess => {
                    let handle = match self.machine.processes().start(plan.process.clone()).await {
                        Ok(handle) => handle,
                        Err(error) => {
                            self.mark_failed(model)?;
                            return Err(error.into());
                        }
                    };
                    self.workloads
                        .get_mut(model)
                        .expect("workload must exist")
                        .process = Some(handle);
                    self.observe(model, ReconcileEvidence::ProcessStarted)?;
                }
                ReconcileAction::VerifyReadiness => {
                    let target = plan.readiness.clone();
                    if let Err(error) = self.machine.readiness().wait_ready(target.clone()).await {
                        self.mark_failed(model)?;
                        return Err(error.into());
                    }
                    match self.machine.health().is_healthy(target).await {
                        Ok(true) => {}
                        Ok(false) => {
                            self.mark_failed(model)?;
                            anyhow::bail!("recovered runtime is unhealthy");
                        }
                        Err(error) => {
                            self.mark_failed(model)?;
                            return Err(error.into());
                        }
                    }
                    self.observe(model, ReconcileEvidence::ReadinessVerified)?;
                }
                ReconcileAction::AttachToExistingRouter => return Ok(()),
                other => anyhow::bail!("unexpected recovery action for '{model}': {other:?}"),
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
        FakeReadinessProbe, FakeRuntimeAdapter, FakeRuntimePreparer,
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
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Failed)
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
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Ready)
        );
        assert_eq!(
            orchestrator
                .workload_plan("qwen-4b")
                .unwrap()
                .local_endpoint,
            "http://127.0.0.1:39122"
        );
    }

    #[tokio::test]
    async fn two_models_own_independent_workloads() {
        let mut orchestrator = fake_orchestrator();

        orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();
        orchestrator
            .record_attachment("qwen-4b", LocalRouteAttachmentId(101))
            .unwrap();

        let qwen_plan = orchestrator.workload_plan("qwen-4b").cloned().unwrap();
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Routable)
        );

        orchestrator
            .prepare_until_ready(ModelDemand::new("deepseek-8b").unwrap())
            .await
            .unwrap();

        assert_eq!(orchestrator.workload_count(), 2);
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Routable)
        );
        assert_eq!(
            orchestrator.workload_attachment_id("qwen-4b"),
            Some(LocalRouteAttachmentId(101))
        );
        assert_eq!(orchestrator.workload_plan("qwen-4b"), Some(&qwen_plan));
        assert_eq!(
            orchestrator.workload_state("deepseek-8b"),
            Some(NodeState::Ready)
        );
        assert_eq!(orchestrator.workload_attachment_id("deepseek-8b"), None);
    }

    #[tokio::test]
    async fn repeated_same_model_demand_reuses_one_workload() {
        let mut orchestrator = fake_orchestrator();

        orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();
        assert_eq!(orchestrator.workload_count(), 1);

        orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();

        assert_eq!(orchestrator.workload_count(), 1);
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Ready)
        );
    }

    #[tokio::test]
    async fn repeated_routable_demand_returns_existing_attachment() {
        let mut orchestrator = fake_orchestrator();

        orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();
        orchestrator
            .record_attachment("qwen-4b", LocalRouteAttachmentId(101))
            .unwrap();

        let outcome = orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();

        assert_eq!(
            outcome,
            LocalPreparationOutcome::Routable(LocalRouteAttachmentId(101))
        );
        assert_eq!(orchestrator.workload_count(), 1);
        assert_eq!(
            orchestrator.workload_attachment_id("qwen-4b"),
            Some(LocalRouteAttachmentId(101))
        );
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Routable)
        );
    }

    #[tokio::test]
    async fn interrupted_preparing_artifact_reuses_persisted_resolution_receipt() {
        let mut orchestrator = fake_orchestrator();
        let model = "qwen-4b";

        assert_eq!(
            orchestrator.next_action(model).unwrap(),
            ReconcileAction::Resolve
        );
        orchestrator
            .workloads
            .get_mut(model)
            .expect("workload")
            .resolved = Some(ResolvedModel {
            model: model.into(),
            artifact_source: "qwen/fake.gguf".into(),
            artifact_digest: Some("sha256:fake".into()),
            runtime: "llama.cpp".into(),
            runtime_version: Some("fake-v0".into()),
        });
        orchestrator
            .observe(model, ReconcileEvidence::Resolved)
            .unwrap();

        assert_eq!(
            orchestrator.workload_state(model),
            Some(NodeState::PreparingArtifact)
        );

        // A new orchestration call must continue from the workload receipt,
        // not depend on a previous function-local variable.
        assert_eq!(
            orchestrator
                .prepare_until_ready(ModelDemand::new(model).unwrap())
                .await
                .unwrap(),
            LocalPreparationOutcome::Ready
        );
        assert_eq!(orchestrator.workload_state(model), Some(NodeState::Ready));
    }

    #[tokio::test]
    async fn interrupted_preparing_runtime_reuses_persisted_artifact_receipt() {
        let mut orchestrator = fake_orchestrator();
        let model = "qwen-4b";

        orchestrator.next_action(model).unwrap();
        {
            let workload = orchestrator.workloads.get_mut(model).expect("workload");
            workload.resolved = Some(ResolvedModel {
                model: model.into(),
                artifact_source: "qwen/fake.gguf".into(),
                artifact_digest: Some("sha256:fake".into()),
                runtime: "llama.cpp".into(),
                runtime_version: Some("fake-v0".into()),
            });
        }
        orchestrator
            .observe(model, ReconcileEvidence::Resolved)
            .unwrap();
        orchestrator
            .workloads
            .get_mut(model)
            .expect("workload")
            .artifact = Some(PreparedArtifact {
            local_path: "/fake/artifacts/qwen_fake.gguf".into(),
            verified: true,
        });
        orchestrator
            .observe(model, ReconcileEvidence::ArtifactPrepared)
            .unwrap();

        assert_eq!(
            orchestrator.next_action(model).unwrap(),
            ReconcileAction::PrepareRuntime
        );
        assert_eq!(
            orchestrator.workload_state(model),
            Some(NodeState::PreparingRuntime)
        );

        assert_eq!(
            orchestrator
                .prepare_until_ready(ModelDemand::new(model).unwrap())
                .await
                .unwrap(),
            LocalPreparationOutcome::Ready
        );
        assert_eq!(orchestrator.workload_state(model), Some(NodeState::Ready));
    }

    #[tokio::test]
    async fn interrupted_starting_reuses_persisted_runtime_and_artifact_receipts() {
        let mut orchestrator = fake_orchestrator();
        let model = "qwen-4b";

        orchestrator.next_action(model).unwrap();
        {
            let workload = orchestrator.workloads.get_mut(model).expect("workload");
            workload.resolved = Some(ResolvedModel {
                model: model.into(),
                artifact_source: "qwen/fake.gguf".into(),
                artifact_digest: Some("sha256:fake".into()),
                runtime: "llama.cpp".into(),
                runtime_version: Some("fake-v0".into()),
            });
        }
        orchestrator
            .observe(model, ReconcileEvidence::Resolved)
            .unwrap();
        orchestrator
            .workloads
            .get_mut(model)
            .expect("workload")
            .artifact = Some(PreparedArtifact {
            local_path: "/fake/artifacts/qwen_fake.gguf".into(),
            verified: true,
        });
        orchestrator
            .observe(model, ReconcileEvidence::ArtifactPrepared)
            .unwrap();
        orchestrator.next_action(model).unwrap();
        orchestrator
            .workloads
            .get_mut(model)
            .expect("workload")
            .runtime = Some(PreparedRuntime {
            executable: "/fake/runtime/llama.cpp/server".into(),
        });
        orchestrator
            .observe(model, ReconcileEvidence::RuntimePrepared)
            .unwrap();

        assert_eq!(
            orchestrator.workload_state(model),
            Some(NodeState::Starting)
        );

        assert_eq!(
            orchestrator
                .prepare_until_ready(ModelDemand::new(model).unwrap())
                .await
                .unwrap(),
            LocalPreparationOutcome::Ready
        );
        assert_eq!(orchestrator.workload_state(model), Some(NodeState::Ready));
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
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::LocalUnsupported)
        );
        assert_eq!(orchestrator.workload_plan("qwen-4b"), None);
        assert_eq!(orchestrator.workload_attachment_id("qwen-4b"), None);

        let repeated = orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();
        assert!(matches!(repeated, LocalPreparationOutcome::Unsupported(_)));
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::LocalUnsupported)
        );
    }
}
