use burncloud_node_runtime::{DemandReconciler, InvalidNodeTransition, ReconcileEvidence};
use burncloud_router::{
    LocalRouteAttacher, LocalRouteAttachment, LocalRouteAttachmentError, LocalRouteAttachmentId,
};

#[derive(Debug, thiserror::Error)]
pub enum NodeRouteAttachmentError {
    #[error(transparent)]
    Attachment(#[from] LocalRouteAttachmentError),
    #[error(transparent)]
    Transition(#[from] InvalidNodeTransition),
}

/// Attach a READY Node capability to BurnCloud's existing routing truth.
///
/// `RouterAttached` is evidence of a completed traffic-owned side effect, not
/// an intention. Therefore the reconciler may enter `Routable` only after the
/// attacher has returned the stable channel identity successfully.
pub async fn attach_ready_node_route<A: LocalRouteAttacher>(
    reconciler: &mut DemandReconciler,
    attacher: &A,
    attachment: LocalRouteAttachment,
) -> Result<LocalRouteAttachmentId, NodeRouteAttachmentError> {
    let attachment_id = attacher.attach(attachment).await?;
    reconciler.observe(ReconcileEvidence::RouterAttached)?;
    Ok(attachment_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use burncloud_node_runtime::{NodeState, ReconcileAction};

    struct FakeAttacher {
        result: Result<LocalRouteAttachmentId, &'static str>,
    }

    #[async_trait]
    impl LocalRouteAttacher for FakeAttacher {
        async fn attach(
            &self,
            _attachment: LocalRouteAttachment,
        ) -> Result<LocalRouteAttachmentId, LocalRouteAttachmentError> {
            self.result.map_err(|message| {
                LocalRouteAttachmentError::AttachFailed(message.to_string())
            })
        }

        async fn detach(
            &self,
            _attachment_id: LocalRouteAttachmentId,
        ) -> Result<(), LocalRouteAttachmentError> {
            Ok(())
        }
    }

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

    fn attachment() -> LocalRouteAttachment {
        LocalRouteAttachment {
            model: "fake-model".into(),
            base_url: "http://127.0.0.1:18080".into(),
        }
    }

    #[tokio::test]
    async fn successful_existing_route_attachment_is_required_for_routable() {
        let mut reconciler = ready_reconciler();
        let attacher = FakeAttacher {
            result: Ok(LocalRouteAttachmentId(42)),
        };

        let id = attach_ready_node_route(&mut reconciler, &attacher, attachment())
            .await
            .unwrap();

        assert_eq!(id, LocalRouteAttachmentId(42));
        assert_eq!(reconciler.state(), NodeState::Routable);
        assert!(reconciler.state().is_serving());
    }

    #[tokio::test]
    async fn failed_existing_route_attachment_must_not_become_routable() {
        let mut reconciler = ready_reconciler();
        let attacher = FakeAttacher {
            result: Err("database write failed"),
        };

        let result = attach_ready_node_route(&mut reconciler, &attacher, attachment()).await;

        assert!(result.is_err());
        assert_eq!(reconciler.state(), NodeState::Ready);
        assert!(!reconciler.state().is_serving());
    }
}
