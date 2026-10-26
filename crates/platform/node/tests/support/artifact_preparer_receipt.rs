#![allow(clippy::panic_in_result_fn, reason = "测试断言用于报告回执不变量失败")]

use std::path::PathBuf;

use super::ArtifactPreparer;
use crate::preparation::ArtifactVerificationStatus;

#[test]
fn verified_receipt_keeps_and_absolutizes_the_download_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let preparer = ArtifactPreparer::new();
    let path = PathBuf::from("models/model.gguf");

    let receipt = preparer.build_receipt(path.clone(), true)?;

    assert!(receipt.is_verified());
    assert_eq!(
        receipt.verification_status(),
        ArtifactVerificationStatus::Verified
    );
    assert!(receipt.local_path().is_absolute());
    assert!(receipt.local_path().ends_with(path));
    Ok(())
}

#[test]
fn failed_receipt_discards_the_candidate_path() -> Result<(), Box<dyn std::error::Error>> {
    let preparer = ArtifactPreparer::new();

    let receipt = preparer.build_receipt(PathBuf::from("models/model.gguf"), false)?;

    assert_eq!(
        receipt.verification_status(),
        ArtifactVerificationStatus::Failed
    );
    assert!(!receipt.is_verified());
    assert!(receipt.local_path().as_os_str().is_empty());
    Ok(())
}
