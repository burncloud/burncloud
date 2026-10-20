use std::path::PathBuf;

use super::ArtifactPreparer;
use crate::preparation::ArtifactVerificationStatus;

#[test]
fn verified_receipt_keeps_the_download_path() {
    let preparer = ArtifactPreparer::new();
    let path = PathBuf::from("models/model.gguf");

    let receipt = preparer
        .build_receipt(path.clone(), true)
        .expect("verified receipt should construct");

    assert!(receipt.is_verified());
    assert_eq!(
        receipt.verification_status(),
        ArtifactVerificationStatus::Verified
    );
    assert!(receipt.local_path().is_absolute());
    assert!(receipt.local_path().ends_with(path));
}

#[test]
fn failed_receipt_discards_the_download_path() {
    let preparer = ArtifactPreparer::new();

    let receipt = preparer
        .build_receipt(PathBuf::from("models/model.gguf"), false)
        .expect("failed receipt should construct");

    assert_eq!(
        receipt.verification_status(),
        ArtifactVerificationStatus::Failed
    );
    assert!(!receipt.is_verified());
    assert!(receipt.local_path().as_os_str().is_empty());
}
