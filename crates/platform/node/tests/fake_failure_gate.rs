use async_trait::async_trait;
use burncloud_node_runtime::{
    ArtifactPreparation, ArtifactPrepareError, DemandReconciler, Digest, PreparedArtifact,
    ReconcileEvidence,
};
use url::Url;

struct FailingArtifactPreparer;

#[async_trait]
impl ArtifactPreparation for FailingArtifactPreparer {
    async fn prepare(
        &self,
        _url: Url,
        _expected_digest: Digest,
    ) -> Result<PreparedArtifact, ArtifactPrepareError> {
        Err(ArtifactPrepareError::PrepareFailed(
            "expected fake failure".into(),
        ))
    }
}

#[tokio::test]
async fn failed_artifact_preparation_does_not_emit_success_evidence() {
    let mut r = DemandReconciler::new();
    r.next_action().unwrap();
    r.observe(ReconcileEvidence::Resolved).unwrap();
    let before = r.state();

    let result = FailingArtifactPreparer
        .prepare(
            Url::parse("https://modelscope.cn/models/missing/resolve/main/fake.gguf").unwrap(),
            Digest::parse(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
        )
        .await;

    assert!(result.is_err());
    // Orchestrator has no successful receipt, so it must not observe
    // ArtifactPrepared. The state therefore remains unchanged.
    assert_eq!(r.state(), before);
}
