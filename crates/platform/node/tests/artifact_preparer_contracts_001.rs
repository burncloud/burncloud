use std::error::Error;

use burncloud_node_runtime::{
    ArtifactPreparation, ArtifactPrepareError, ArtifactPreparer, ArtifactVerificationStatus,
    Digest, DigestAlgorithm, PreparedArtifact,
};

fn assert_error_contract<T: Error>() {}

fn assert_preparation_contract<T: ArtifactPreparation>() {}

#[test]
fn preparation_error_is_displayable_and_implements_error() {
    assert_error_contract::<ArtifactPrepareError>();
    let error = ArtifactPrepareError::PrepareFailed("network unavailable".to_string());
    assert!(error.to_string().contains("network unavailable"));
}

#[test]
fn public_node_reexports_include_the_preparation_contract() {
    assert_preparation_contract::<ArtifactPreparer>();
    let artifact = PreparedArtifact::new("models/model.gguf", ArtifactVerificationStatus::Verified)
        .expect("固定测试路径应能构造成功回执");
    assert!(artifact.is_verified());

    let digest = Digest::new(DigestAlgorithm::Sha256, "0".repeat(64)).expect("固定测试摘要应合法");
    assert_eq!(digest.algorithm(), DigestAlgorithm::Sha256);
}
