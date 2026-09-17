use burncloud_node_runtime::{DemandReconciler, ReconcileEvidence};
use std::future::Future;

/// Proof that the exact route attachment was removed before recovery begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetachedRoute<T>(pub T);

/// Execute the traffic-owned attachment side effect and record routing evidence
/// only after it succeeds.
pub async fn attach_ready_node_route<T, F, Fut>(
    reconciler: &mut DemandReconciler,
    attach: F,
) -> anyhow::Result<T>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    let attachment_id = attach().await?;
    reconciler.observe(ReconcileEvidence::RouterAttached)?;
    Ok(attachment_id)
}

/// Fail closed when a routable local runtime becomes unhealthy.
///
/// State is made non-serving first, then Traffic is asked to remove the exact
/// existing-router attachment. Recovery receives a `DetachedRoute` receipt only
/// when that side effect succeeds, so callers cannot accidentally reattach over
/// a route that was never removed.
pub async fn detach_unhealthy_node_route<T, F, Fut>(
    reconciler: &mut DemandReconciler,
    attachment_id: T,
    detach: F,
) -> anyhow::Result<DetachedRoute<T>>
where
    T: Copy,
    F: FnOnce(T) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    reconciler.observe(ReconcileEvidence::BecameUnhealthy)?;
    detach(attachment_id).await?;
    Ok(DetachedRoute(attachment_id))
}

/// Open the recovery rail only after successful route detachment.
pub fn begin_detached_route_recovery<T>(
    reconciler: &mut DemandReconciler,
    _detached: DetachedRoute<T>,
) -> anyhow::Result<()> {
    reconciler.retry()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use burncloud_node_runtime::{NodeState, ReconcileAction};

    fn ready_reconciler() -> DemandReconciler {
        let mut reconciler = DemandReconciler::new();
        assert_eq!(reconciler.next_action().unwrap(), ReconcileAction::Resolve);
        reconciler.observe(ReconcileEvidence::Resolved).unwrap();
        reconciler.observe(ReconcileEvidence::ArtifactPrepared).unwrap();
        assert_eq!(reconciler.next_action().unwrap(), ReconcileAction::PrepareRuntime);
        reconciler.observe(ReconcileEvidence::RuntimePrepared).unwrap();
        reconciler.observe(ReconcileEvidence::ProcessStarted).unwrap();
        reconciler.observe(ReconcileEvidence::ReadinessVerified).unwrap();
        assert_eq!(reconciler.state(), NodeState::Ready);
        reconciler
    }

    fn routable_reconciler() -> DemandReconciler {
        let mut reconciler = ready_reconciler();
        reconciler.observe(ReconcileEvidence::RouterAttached).unwrap();
        assert_eq!(reconciler.state(), NodeState::Routable);
        reconciler
    }

    #[tokio::test]
    async fn successful_existing_route_attachment_is_required_for_routable() {
        let mut reconciler = ready_reconciler();
        let id = attach_ready_node_route(&mut reconciler, || async { Ok(42_i32) }).await.unwrap();
        assert_eq!(id, 42);
        assert_eq!(reconciler.state(), NodeState::Routable);
        assert!(reconciler.state().is_serving());
    }

    #[tokio::test]
    async fn failed_existing_route_attachment_must_not_become_routable() {
        let mut reconciler = ready_reconciler();
        let result: anyhow::Result<i32> = attach_ready_node_route(&mut reconciler, || async {
            anyhow::bail!("database write failed")
        }).await;
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
        }).await.unwrap();
        assert_eq!(reconciler.state(), NodeState::Unhealthy);
        assert!(!reconciler.state().is_serving());

        begin_detached_route_recovery(&mut reconciler, detached).unwrap();
        assert_eq!(reconciler.state(), NodeState::Starting);
        assert_eq!(reconciler.next_action().unwrap(), ReconcileAction::StartProcess);
    }

    #[tokio::test]
    async fn failed_detach_stays_unhealthy_and_cannot_produce_recovery_receipt() {
        let mut reconciler = routable_reconciler();
        let result = detach_unhealthy_node_route(&mut reconciler, 42_i32, |_| async {
            anyhow::bail!("existing router detach failed")
        }).await;
        assert!(result.is_err());
        assert_eq!(reconciler.state(), NodeState::Unhealthy);
        assert!(!reconciler.state().is_serving());
    }
}
