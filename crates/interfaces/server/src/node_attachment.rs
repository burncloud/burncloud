use crate::node_orchestrator::{LocalPreparationOutcome, ModelDemand, NodeOrchestrator};
use crate::node_request::NodeRequestState;
use burncloud_node_runtime::{
    ArtifactPreparer, DemandReconciler, HardwareProbe, HealthProbe, ProcessManager, ReadinessProbe,
    ReconcileEvidence, RuntimeAdapter, RuntimePreparer,
};
use burncloud_service_models::{LocalModelUnsupported, ModelResolver};
use std::future::Future;

/// Proof that the exact route attachment was removed before recovery begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetachedRoute<T>(pub T);

/// Final result of composing local preparation with the existing routing path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalRouteOutcome<T> {
    Routable(T),
    Unsupported(LocalModelUnsupported),
}

/// Execute the traffic-owned attachment side effect and record routing evidence
/// only after it succeeds.
pub async fn attach_ready_node_route<T, F, Fut>(
    reconciler: &mut DemandReconciler,
    model: &str,
    request_state: &NodeRequestState,
    attach: F,
) -> anyhow::Result<T>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    let attachment_id = attach().await?;
    reconciler.observe(ReconcileEvidence::RouterAttached)?;
    request_state.publish(model, reconciler.state());
    Ok(attachment_id)
}

/// Run the local preparation rail and, only after READY, attach its runtime-owned
/// endpoint to BurnCloud's existing routing truth.
///
/// The caller supplies the Traffic-owned side effect. This application seam
/// never creates a second router and never invents a local endpoint.
pub async fn prepare_and_attach_node_route<M, T, H, A, R, P, Q, E, I, F, Fut>(
    orchestrator: &mut NodeOrchestrator<M, T, H, A, R, P, Q, E>,
    demand: ModelDemand,
    attach: F,
) -> anyhow::Result<LocalRouteOutcome<I>>
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
    Fut: Future<Output = anyhow::Result<I>>,
{
    let model = demand.model.clone();
    match orchestrator.prepare_until_ready(demand).await? {
        LocalPreparationOutcome::Unsupported(reason) => Ok(LocalRouteOutcome::Unsupported(reason)),
        LocalPreparationOutcome::Ready => {
            let base_url = orchestrator
                .active_plan()
                .ok_or_else(|| anyhow::anyhow!("ready local runtime has no process plan"))?
                .local_endpoint
                .clone();
            let request_state = orchestrator.request_state();
            let attach_model = model.clone();
            let attachment_id = attach_ready_node_route(
                orchestrator.machine_mut().reconciler_mut(),
                &model,
                &request_state,
                || attach(attach_model, base_url),
            )
            .await?;
            Ok(LocalRouteOutcome::Routable(attachment_id))
        }
    }
}

/// Fail closed when a routable local runtime becomes unhealthy.
///
/// State is made non-serving first, then Traffic is asked to remove the exact
/// existing-router attachment. Recovery receives a `DetachedRoute` receipt only
/// when that side effect succeeds, so callers cannot accidentally reattach over
/// a route that was never removed.
pub async fn detach_unhealthy_node_route<T, F, Fut>(
    reconciler: &mut DemandReconciler,
    model: &str,
    request_state: &NodeRequestState,
    attachment_id: T,
    detach: F,
) -> anyhow::Result<DetachedRoute<T>>
where
    T: Copy,
    F: FnOnce(T) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    reconciler.observe(ReconcileEvidence::BecameUnhealthy)?;
    request_state.publish(model, reconciler.state());
    detach(attachment_id).await?;
    Ok(DetachedRoute(attachment_id))
}

/// Open the recovery rail only after successful route detachment.
pub fn begin_detached_route_recovery<T>(
    reconciler: &mut DemandReconciler,
    model: &str,
    request_state: &NodeRequestState,
    _detached: DetachedRoute<T>,
) -> anyhow::Result<()> {
    reconciler.retry()?;
    request_state.publish(model, reconciler.state());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use burncloud_node_runtime::{
        FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe, FakeProcessManager,
        FakeReadinessProbe, FakeRuntimeAdapter, FakeRuntimePreparer, NodeComposition, NodeState,
        ReconcileAction,
    };
    use burncloud_service_models::FakeModelResolver;

    fn ready_reconciler() -> DemandReconciler {
        let mut reconciler = DemandReconciler::new();
        assert_eq!(reconciler.next_action().unwrap(), ReconcileAction::Resolve);
        reconciler.observe(ReconcileEvidence::Resolved).unwrap();
        reconciler
            .observe(ReconcileEvidence::ArtifactPrepared)
            .unwrap();
        assert_eq!(
            reconciler.next_action().unwrap(),
            ReconcileAction::PrepareRuntime
        );
        reconciler
            .observe(ReconcileEvidence::RuntimePrepared)
            .unwrap();
        reconciler
            .observe(ReconcileEvidence::ProcessStarted)
            .unwrap();
        reconciler
            .observe(ReconcileEvidence::ReadinessVerified)
            .unwrap();
        assert_eq!(reconciler.state(), NodeState::Ready);
        reconciler
    }

    fn routable_reconciler() -> DemandReconciler {
        let mut reconciler = ready_reconciler();
        reconciler
            .observe(ReconcileEvidence::RouterAttached)
            .unwrap();
        assert_eq!(reconciler.state(), NodeState::Routable);
        reconciler
    }

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

    #[tokio::test]
    async fn successful_existing_route_attachment_is_required_for_routable() {
        let mut reconciler = ready_reconciler();
        let id = attach_ready_node_route(&mut reconciler, || async { Ok(42_i32) })
            .await
            .unwrap();
        assert_eq!(id, 42);
        assert_eq!(reconciler.state(), NodeState::Routable);
        assert!(reconciler.state().is_serving());
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
                Ok(77_i32)
            },
        )
        .await
        .unwrap();

        assert_eq!(outcome, LocalRouteOutcome::Routable(77));
        assert_eq!(
            orchestrator.machine().reconciler().state(),
            NodeState::Routable
        );
        assert!(orchestrator.machine().reconciler().state().is_serving());
    }

    #[tokio::test]
    async fn failed_existing_route_attachment_must_not_become_routable() {
        let mut reconciler = ready_reconciler();
        let result: anyhow::Result<i32> = attach_ready_node_route(&mut reconciler, || async {
            anyhow::bail!("database write failed")
        })
        .await;
        assert!(result.is_err());
        assert_eq!(reconciler.state(), NodeState::Ready);
        assert!(!reconciler.state().is_serving());
    }

    #[tokio::test]
    async fn unhealthy_route_becomes_non_serving_before_detach_and_can_then_recover() {
        let mut reconciler = routable_reconciler();
        let detached = detach_unhealthy_node_route(&mut reconciler, 42_i32, |id| async move {
            assert_eq!(id, 42);
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(reconciler.state(), NodeState::Unhealthy);
        assert!(!reconciler.state().is_serving());

        begin_detached_route_recovery(&mut reconciler, detached).unwrap();
        assert_eq!(reconciler.state(), NodeState::Starting);
        assert_eq!(
            reconciler.next_action().unwrap(),
            ReconcileAction::StartProcess
        );
    }

    #[tokio::test]
    async fn failed_detach_stays_unhealthy_and_cannot_produce_recovery_receipt() {
        let mut reconciler = routable_reconciler();
        let result = detach_unhealthy_node_route(&mut reconciler, 42_i32, |_| async {
            anyhow::bail!("existing router detach failed")
        })
        .await;
        assert!(result.is_err());
        assert_eq!(reconciler.state(), NodeState::Unhealthy);
        assert!(!reconciler.state().is_serving());
    }
}
