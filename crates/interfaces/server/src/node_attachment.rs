use burncloud_node_runtime::{DemandReconciler, ReconcileEvidence};
use std::future::Future;

/// Execute the traffic-owned attachment side effect and record routing evidence
/// only after it succeeds.
///
/// `RouterAttached` is proof of a completed side effect, not an intention. The
/// closure is the application-layer seam: production passes the existing-router
/// adapter here; tests can prove failure never advances the state to Routable.
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

#[cfg(test)]
mod tests {
    use super::*;
    use burncloud_node_runtime::{NodeState, ReconcileAction};

    fn ready_reconciler() -> DemandReconciler {
        let mut reconciler = DemandReconciler::new();
        assert_eq!(reconciler.next_action().unwrap(), ReconcileAction::Resolve);
        reconciler.observe(ReconcileEvidence::Resolved).unwrap();
        reconciler.observe(ReconcileEvidence::ArtifactPrepared).unwrap();
        assert_eq!(
            reconciler.next_action().unwrap(),
            ReconcileAction::PrepareRuntime
        );
        reconciler.observe(ReconcileEvidence::RuntimePrepared).unwrap();
        reconciler.observe(ReconcileEvidence::ProcessStarted).unwrap();
        reconciler.observe(ReconcileEvidence::ReadinessVerified).unwrap();
        assert_eq!(reconciler.state(), NodeState::Ready);
        reconciler
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
}
