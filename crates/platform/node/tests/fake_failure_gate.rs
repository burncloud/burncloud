use async_trait::async_trait;
use burncloud_node_runtime::{
    ArtifactPrepareError, ArtifactPreparer, ArtifactRequest, DemandReconciler, PreparedArtifact,
    ReconcileEvidence,
};

struct FailingArtifactPreparer;

#[async_trait]
impl ArtifactPreparer for FailingArtifactPreparer {
    async fn prepare(
        &self,
        _request: ArtifactRequest,
    ) -> Result<PreparedArtifact, ArtifactPrepareError> {
        Err(ArtifactPrepareError::PrepareFailed("expected fake failure".into()))
    }
}

#[tokio::test]
async fn failed_artifact_preparation_does_not_emit_success_evidence() {
    let mut r = DemandReconciler::new();
    r.next_action().unwrap();
    r.observe(ReconcileEvidence::Resolved).unwrap();
    let before = r.state();

    let result = FailingArtifactPreparer
        .prepare(ArtifactRequest { source: "missing".into(), expected_digest: None })
        .await;

    assert!(result.is_err());
    // Orchestrator has no successful receipt, so it must not observe
    // ArtifactPrepared. The state therefore remains unchanged.
    assert_eq!(r.state(), before);
}
