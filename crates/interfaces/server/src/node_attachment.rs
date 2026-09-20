use crate::node_orchestrator::{LocalPreparationOutcome, ModelDemand, NodeOrchestrator};
use burncloud_node_runtime::{
    ArtifactPreparer, HardwareProbe, HealthProbe, ProcessManager, ReadinessProbe, RuntimeAdapter,
    RuntimePreparer,
};
use burncloud_router::local_attachment::LocalRouteAttachmentId;
use burncloud_service_models::{LocalModelUnsupported, ModelResolver};
use std::future::Future;

/// Proof that the exact route attachment was removed before recovery begins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetachedRoute {
    model: String,
    attachment_id: LocalRouteAttachmentId,
}

impl DetachedRoute {
    pub fn model(&self) -> &str {
        &self.model
    }

    pub const fn attachment_id(&self) -> LocalRouteAttachmentId {
        self.attachment_id
    }
}

/// Final result of composing local preparation with the existing routing path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalRouteOutcome {
    Routable(LocalRouteAttachmentId),
    Unsupported(LocalModelUnsupported),
}

/// Execute the Traffic-owned attachment side effect and record routing evidence
/// only after it succeeds.
///
/// If Traffic attachment fails, READY must not become a permanent pseudo-progress
/// state: the model workload converges to Failed and request-side
/// MODEL_PREPARING stops.
pub async fn attach_ready_node_route<M, T, H, A, R, P, Q, E, F, Fut>(
    orchestrator: &mut NodeOrchestrator<M, T, H, A, R, P, Q, E>,
    model: &str,
    attach: F,
) -> anyhow::Result<LocalRouteAttachmentId>
where
    M: ModelResolver,
    T: RuntimeAdapter,
    H: HardwareProbe,
    A: ArtifactPreparer,
    R: RuntimePreparer,
    P: ProcessManager,
    Q: ReadinessProbe,
    E: HealthProbe,
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<LocalRouteAttachmentId>>,
{
    // Validate the lifecycle gate before creating any Traffic side effect.
    orchestrator.ensure_ready_for_attachment(model)?;

    let attachment_id = match attach().await {
        Ok(attachment_id) => attachment_id,
        Err(error) => {
            orchestrator.mark_failed(model)?;
            return Err(error);
        }
    };
    orchestrator.record_attachment(model, attachment_id)?;
    Ok(attachment_id)
}

/// Run the local preparation rail and, only after READY, attach its runtime-owned
/// endpoint to BurnCloud's existing routing truth.
///
/// The caller supplies the Traffic-owned side effect. This application seam
/// never creates a second router and never invents a local endpoint.
pub async fn prepare_and_attach_node_route<M, T, H, A, R, P, Q, E, F, Fut>(
    orchestrator: &mut NodeOrchestrator<M, T, H, A, R, P, Q, E>,
    demand: ModelDemand,
    attach: F,
) -> anyhow::Result<LocalRouteOutcome>
where
    M: ModelResolver,
    T: RuntimeAdapter,
    H: HardwareProbe,
    A: ArtifactPreparer,
    R: RuntimePreparer,
    P: ProcessManager,
    Q: ReadinessProbe,
    E: HealthProbe,
    F: FnOnce(String, String) -> Fut,
    Fut: Future<Output = anyhow::Result<LocalRouteAttachmentId>>,
{
    let model = demand.model.clone();
    match orchestrator.prepare_until_ready(demand).await? {
        LocalPreparationOutcome::Unsupported(reason) => Ok(LocalRouteOutcome::Unsupported(reason)),
        LocalPreparationOutcome::Routable(attachment_id) => {
            Ok(LocalRouteOutcome::Routable(attachment_id))
        }
        LocalPreparationOutcome::Ready => {
            let base_url = orchestrator
                .workload_plan(&model)
                .ok_or_else(|| anyhow::anyhow!("ready local runtime has no process plan"))?
                .local_endpoint
                .clone();
            let attach_model = model.clone();
            let attachment_id =
                attach_ready_node_route(orchestrator, &model, || attach(attach_model, base_url))
                    .await?;
            Ok(LocalRouteOutcome::Routable(attachment_id))
        }
    }
}

/// Fail closed when a routable local runtime becomes unhealthy.
///
/// Traffic quarantine is deliberately first. Only after Existing ModelRouter can
/// no longer discover/select the local channel do we publish Unhealthy and try
/// exact deletion. A failed delete therefore leaves a disabled, non-routable
/// channel and cannot manufacture a recovery receipt.
pub async fn detach_unhealthy_node_route<M, T, H, A, R, P, Q, E, FQ, QFut, FD, DFut>(
    orchestrator: &mut NodeOrchestrator<M, T, H, A, R, P, Q, E>,
    model: &str,
    attachment_id: LocalRouteAttachmentId,
    quarantine: FQ,
    detach: FD,
) -> anyhow::Result<DetachedRoute>
where
    M: ModelResolver,
    T: RuntimeAdapter,
    H: HardwareProbe,
    A: ArtifactPreparer,
    R: RuntimePreparer,
    P: ProcessManager,
    Q: ReadinessProbe,
    E: HealthProbe,
    FQ: FnOnce(LocalRouteAttachmentId) -> QFut,
    QFut: Future<Output = anyhow::Result<()>>,
    FD: FnOnce(LocalRouteAttachmentId) -> DFut,
    DFut: Future<Output = anyhow::Result<()>>,
{
    if orchestrator.workload_attachment_id(model) != Some(attachment_id) {
        anyhow::bail!(
            "attachment {} does not belong to model '{model}'",
            attachment_id.0
        );
    }
    if orchestrator.workload_state(model) != Some(burncloud_node_runtime::NodeState::Routable) {
        anyhow::bail!("model '{model}' is not routable");
    }

    // Critical ordering: Traffic truth first, lifecycle state second.
    quarantine(attachment_id).await?;
    orchestrator.mark_unhealthy(model)?;

    // Delete is cleanup. If it fails, Traffic is already fail-closed.
    detach(attachment_id).await?;
    orchestrator.clear_attachment(model, attachment_id)?;
    Ok(DetachedRoute {
        model: model.to_string(),
        attachment_id,
    })
}

/// Open the recovery rail only after successful route detachment.
pub fn begin_detached_route_recovery<M, T, H, A, R, P, Q, E>(
    orchestrator: &mut NodeOrchestrator<M, T, H, A, R, P, Q, E>,
    model: &str,
    detached: DetachedRoute,
) -> anyhow::Result<()>
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
    if detached.model() != model {
        anyhow::bail!(
            "detached route receipt belongs to model '{}', not '{model}'",
            detached.model()
        );
    }
    if orchestrator.workload_attachment_id(model).is_some() {
        anyhow::bail!("model '{model}' still owns a route attachment");
    }
    orchestrator.begin_recovery(model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_request::NodeRequestState;
    use burncloud_node_runtime::{
        FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe, FakeProcessManager,
        FakeReadinessProbe, FakeRuntimeAdapter, FakeRuntimePreparer, NodeComposition, NodeState,
    };
    use burncloud_service_models::FakeModelResolver;

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

    async fn ready_orchestrator() -> (
        NodeOrchestrator<
            FakeModelResolver,
            FakeRuntimeAdapter,
            FakeHardwareProbe,
            FakeArtifactPreparer,
            FakeRuntimePreparer,
            FakeProcessManager,
            FakeReadinessProbe,
            FakeHealthProbe,
        >,
        NodeRequestState,
    ) {
        let request_state = NodeRequestState::default();
        let mut orchestrator = fake_orchestrator().with_request_state(request_state.clone());
        orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();
        (orchestrator, request_state)
    }

    #[tokio::test]
    async fn successful_existing_route_attachment_is_required_for_routable() {
        let (mut orchestrator, request_state) = ready_orchestrator().await;
        let id = attach_ready_node_route(&mut orchestrator, "qwen-4b", || async {
            Ok(LocalRouteAttachmentId(42))
        })
        .await
        .unwrap();

        assert_eq!(id, LocalRouteAttachmentId(42));
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Routable)
        );
        assert_eq!(
            orchestrator.workload_attachment_id("qwen-4b"),
            Some(LocalRouteAttachmentId(42))
        );
        assert_eq!(
            request_state.state_for("qwen-4b"),
            Some(NodeState::Routable)
        );
    }

    #[tokio::test]
    async fn ready_runtime_is_automatically_composed_with_existing_route_attachment() {
        let mut orchestrator = fake_orchestrator();
        let outcome = prepare_and_attach_node_route(
            &mut orchestrator,
            ModelDemand::new("qwen-4b").unwrap(),
            |model, base_url| async move {
                assert_eq!(model, "qwen-4b");
                assert_eq!(base_url, "http://127.0.0.1:39122");
                Ok(LocalRouteAttachmentId(77))
            },
        )
        .await
        .unwrap();

        assert_eq!(
            outcome,
            LocalRouteOutcome::Routable(LocalRouteAttachmentId(77))
        );
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Routable)
        );
    }

    #[tokio::test]
    async fn non_ready_model_cannot_create_attachment_side_effect() {
        let mut orchestrator = fake_orchestrator();
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let calls_for_attach = calls.clone();

        let result = attach_ready_node_route(&mut orchestrator, "qwen-4b", move || async move {
            calls_for_attach.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(LocalRouteAttachmentId(999))
        })
        .await;

        assert!(result.is_err());
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert_eq!(orchestrator.workload_state("qwen-4b"), None);
        assert_eq!(orchestrator.workload_attachment_id("qwen-4b"), None);
    }

    #[tokio::test]
    async fn failed_existing_route_attachment_converges_to_failed() {
        let (mut orchestrator, request_state) = ready_orchestrator().await;
        let result = attach_ready_node_route(&mut orchestrator, "qwen-4b", || async {
            anyhow::bail!("database write failed")
        })
        .await;

        assert!(result.is_err());
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Failed)
        );
        assert_eq!(request_state.state_for("qwen-4b"), Some(NodeState::Failed));
        assert!(request_state.route_miss_response("qwen-4b").is_none());
        assert_eq!(orchestrator.workload_attachment_id("qwen-4b"), None);
    }

    #[tokio::test]
    async fn unhealthy_route_is_quarantined_before_detach_and_can_then_recover() {
        let (mut orchestrator, request_state) = ready_orchestrator().await;
        let route = attach_ready_node_route(&mut orchestrator, "qwen-4b", || async {
            Ok(LocalRouteAttachmentId(42))
        })
        .await
        .unwrap();

        let quarantined = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let quarantine_flag = quarantined.clone();
        let detach_flag = quarantined.clone();
        let detached = detach_unhealthy_node_route(
            &mut orchestrator,
            "qwen-4b",
            route,
            move |id| async move {
                assert_eq!(id, LocalRouteAttachmentId(42));
                quarantine_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            },
            move |id| async move {
                assert_eq!(id, LocalRouteAttachmentId(42));
                assert!(detach_flag.load(std::sync::atomic::Ordering::SeqCst));
                Ok(())
            },
        )
        .await
        .unwrap();

        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Unhealthy)
        );
        assert_eq!(
            request_state.state_for("qwen-4b"),
            Some(NodeState::Unhealthy)
        );
        assert_eq!(orchestrator.workload_attachment_id("qwen-4b"), None);

        begin_detached_route_recovery(&mut orchestrator, "qwen-4b", detached).unwrap();
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Starting)
        );
    }

    #[tokio::test]
    async fn failed_detach_stays_unhealthy_and_cannot_produce_recovery_receipt() {
        let (mut orchestrator, request_state) = ready_orchestrator().await;
        let route = attach_ready_node_route(&mut orchestrator, "qwen-4b", || async {
            Ok(LocalRouteAttachmentId(42))
        })
        .await
        .unwrap();

        let result = detach_unhealthy_node_route(
            &mut orchestrator,
            "qwen-4b",
            route,
            |_| async { Ok(()) },
            |_| async { anyhow::bail!("existing router detach failed") },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Unhealthy)
        );
        assert_eq!(
            request_state.state_for("qwen-4b"),
            Some(NodeState::Unhealthy)
        );
        assert_eq!(
            orchestrator.workload_attachment_id("qwen-4b"),
            Some(LocalRouteAttachmentId(42))
        );
    }

    #[tokio::test]
    async fn detached_route_receipt_cannot_recover_another_model() {
        let mut orchestrator = fake_orchestrator();

        orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();
        let qwen_route = attach_ready_node_route(&mut orchestrator, "qwen-4b", || async {
            Ok(LocalRouteAttachmentId(41))
        })
        .await
        .unwrap();
        let qwen_receipt = detach_unhealthy_node_route(
            &mut orchestrator,
            "qwen-4b",
            qwen_route,
            |_| async { Ok(()) },
            |_| async { Ok(()) },
        )
        .await
        .unwrap();

        orchestrator
            .prepare_until_ready(ModelDemand::new("deepseek-8b").unwrap())
            .await
            .unwrap();
        let deepseek_route =
            attach_ready_node_route(&mut orchestrator, "deepseek-8b", || async {
                Ok(LocalRouteAttachmentId(42))
            })
            .await
            .unwrap();
        let deepseek_receipt = detach_unhealthy_node_route(
            &mut orchestrator,
            "deepseek-8b",
            deepseek_route,
            |_| async { Ok(()) },
            |_| async { Ok(()) },
        )
        .await
        .unwrap();

        assert_eq!(
            orchestrator.workload_state("deepseek-8b"),
            Some(NodeState::Unhealthy)
        );

        let result =
            begin_detached_route_recovery(&mut orchestrator, "deepseek-8b", qwen_receipt);
        assert!(result.is_err());
        assert_eq!(
            orchestrator.workload_state("deepseek-8b"),
            Some(NodeState::Unhealthy)
        );

        begin_detached_route_recovery(&mut orchestrator, "deepseek-8b", deepseek_receipt)
            .unwrap();
        assert_eq!(
            orchestrator.workload_state("deepseek-8b"),
            Some(NodeState::Starting)
        );
    }

    #[tokio::test]
    async fn quarantine_failure_stops_before_unhealthy_or_recovery() {
        let (mut orchestrator, _) = ready_orchestrator().await;
        let route = attach_ready_node_route(&mut orchestrator, "qwen-4b", || async {
            Ok(LocalRouteAttachmentId(42))
        })
        .await
        .unwrap();

        let result = detach_unhealthy_node_route(
            &mut orchestrator,
            "qwen-4b",
            route,
            |_| async { anyhow::bail!("traffic quarantine failed") },
            |_| async { panic!("detach must not run after quarantine failure") },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Routable)
        );
        assert_eq!(
            orchestrator.workload_attachment_id("qwen-4b"),
            Some(LocalRouteAttachmentId(42))
        );
    }
}
