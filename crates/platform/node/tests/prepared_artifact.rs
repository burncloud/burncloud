use std::io::ErrorKind;

use burncloud_node_runtime::{ArtifactVerificationStatus, PreparedArtifact};

#[test]
fn verified_relative_path_becomes_absolute_and_remains_usable() {
    let artifact = PreparedArtifact::new("models/model.gguf", ArtifactVerificationStatus::Verified)
        .expect("verified path should construct");

    assert!(artifact.local_path().is_absolute());
    assert_eq!(
        artifact.verification_status(),
        ArtifactVerificationStatus::Verified
    );
    assert!(artifact.is_verified());
}

#[test]
fn failed_receipt_has_an_empty_path_and_is_not_verified() {
    let artifact = PreparedArtifact::new("", ArtifactVerificationStatus::Failed)
        .expect("empty failed receipt should construct");

    assert!(artifact.local_path().as_os_str().is_empty());
    assert_eq!(
        artifact.verification_status(),
        ArtifactVerificationStatus::Failed
    );
    assert!(!artifact.is_verified());
    assert_eq!(artifact.to_string(), " (verification: failed)");
}

#[test]
fn constructor_rejects_invalid_path_and_status_combinations() {
    let failed_with_path = PreparedArtifact::new("model.gguf", ArtifactVerificationStatus::Failed)
        .expect_err("failed receipt must not contain a path");
    assert_eq!(failed_with_path.kind(), ErrorKind::InvalidInput);

    let verified_without_path = PreparedArtifact::new("", ArtifactVerificationStatus::Verified)
        .expect_err("verified receipt must contain a path");
    assert_eq!(verified_without_path.kind(), ErrorKind::InvalidInput);
}

#[test]
fn display_reports_verified_status_and_path() {
    let path = std::env::temp_dir().join("model.gguf");
    let artifact = PreparedArtifact::new(&path, ArtifactVerificationStatus::Verified)
        .expect("verified path should construct");

    assert_eq!(
        artifact.to_string(),
        format!("{} (verification: verified)", path.display())
    );
}
