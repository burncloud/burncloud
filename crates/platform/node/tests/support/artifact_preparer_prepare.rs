use super::ArtifactPreparer;
use crate::preparation::{ArtifactPrepareError, Digest, DigestAlgorithm};
use url::Url;

#[tokio::test]
async fn rejects_unsupported_sources_before_starting_the_download_manager() {
    let preparer = ArtifactPreparer::new();
    let digest = Digest::new(DigestAlgorithm::Sha256, "0".repeat(64))
        .expect("zero digest should have the expected shape");

    let error = preparer
        .prepare(
            Url::parse("https://example.com/models/example/resolve/main/model.gguf")
                .expect("test URL should parse"),
            digest,
        )
        .await
        .expect_err("unsupported host should be rejected");

    assert!(matches!(error, ArtifactPrepareError::PrepareFailed(_)));
    let lifecycle = preparer.lifecycle.lock().await;
    assert_eq!(lifecycle.ref_count, 0);
    assert!(lifecycle.download_manager.is_none());
}
