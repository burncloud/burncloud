use std::path::PathBuf;

use super::ArtifactPreparer;
use crate::preparation::{ArtifactPrepareError, Digest, DigestAlgorithm};

const CONTENT: &[u8] = b"hello";

#[tokio::test]
async fn verifies_all_supported_digest_algorithms_without_loading_the_whole_file() {
    let path = temporary_path();
    tokio::fs::write(&path, CONTENT)
        .await
        .expect("test artifact should be written");
    let preparer = ArtifactPreparer::new();
    let expected = [
        (
            DigestAlgorithm::Md5,
            "5d41402abc4b2a76b9719d911017c592",
        ),
        (
            DigestAlgorithm::Sha1,
            "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d",
        ),
        (
            DigestAlgorithm::Sha256,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
        ),
        (
            DigestAlgorithm::Sha384,
            "59e1748777448c69de6b800d7a33bbfb9ff1b463e44354c3553bcdb9c666fa90125a3c79f90397bdf5f6a13de828684f",
        ),
        (
            DigestAlgorithm::Sha512,
            "9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca72323c3d99ba5c11d7c7acc6e14b8c5da0c4663475c2e5c3adef46f73bcdec043",
        ),
    ];

    for (algorithm, value) in expected {
        let digest = Digest::new(algorithm, value).expect("known digest should parse");
        assert!(preparer
            .verify_file_digest(&path, &digest)
            .await
            .expect("digest calculation should succeed"));

        let wrong = Digest::new(algorithm, "0".repeat(algorithm.expected_hex_length()))
            .expect("zero digest should have the expected shape");
        assert!(!preparer
            .verify_file_digest(&path, &wrong)
            .await
            .expect("mismatched digest calculation should succeed"));
    }

    tokio::fs::remove_file(path)
        .await
        .expect("test artifact should be removed");
}

#[tokio::test]
async fn returns_prepare_failed_when_the_file_cannot_be_opened() {
    let preparer = ArtifactPreparer::new();
    let digest = Digest::new(DigestAlgorithm::Sha256, "0".repeat(64))
        .expect("zero digest should have the expected shape");

    let error = preparer
        .verify_file_digest(&temporary_path(), &digest)
        .await
        .expect_err("missing artifact should fail");

    assert!(matches!(error, ArtifactPrepareError::PrepareFailed(_)));
}

fn temporary_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "burncloud-artifact-digest-{}.bin",
        std::process::id()
    ))
}
