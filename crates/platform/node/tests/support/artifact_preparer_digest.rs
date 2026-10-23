#![allow(clippy::panic_in_result_fn, reason = "测试断言用于报告摘要校验失败")]

use std::path::PathBuf;

use super::ArtifactPreparer;
use crate::preparation::{ArtifactPrepareError, Digest, DigestAlgorithm};

const CONTENT_LENGTH: usize = 128 * 1024 + 17;
const CONTENT_BYTE: u8 = 0x5a;
const DIGESTS: [(DigestAlgorithm, &str); 5] = [
    (DigestAlgorithm::Md5, "0cde602fd99c10336cbda2e61ae8ebb9"),
    (DigestAlgorithm::Sha1, "0e2d5ca58c38d8d7add2f0cd7285ff1baf6c06f4"),
    (
        DigestAlgorithm::Sha256,
        "5a58a1e2c15da6e63673dcebcae452797792758e89d08a41f2c26a8eb1971e8b",
    ),
    (
        DigestAlgorithm::Sha384,
        "ba3a244a2d705f5a6546d54ca8abe5ba0d062f3da10919ffa46106253ee256df5096d743e7161d5f477aaa5ef2341102",
    ),
    (
        DigestAlgorithm::Sha512,
        "ee8ee7bbe9856310248725821976ba0b3fbf05df347d2859d809905fd298f1bf89ba5e4021e990162103a3d99193cf3837aa8f664781d8be19ed72c16b2e0e39",
    ),
];

#[tokio::test]
async fn verifies_all_algorithms_over_multiple_read_chunks(
) -> Result<(), Box<dyn std::error::Error>> {
    let path = temporary_path();
    let content = vec![CONTENT_BYTE; CONTENT_LENGTH];
    tokio::fs::write(&path, content).await?;

    let preparer = ArtifactPreparer::new();
    for (algorithm, value) in DIGESTS {
        let expected = Digest::new(algorithm, value)?;
        assert!(preparer.verify_file_digest(&path, &expected).await?);

        let wrong = Digest::new(algorithm, "0".repeat(algorithm.expected_hex_length()))?;
        assert!(!preparer.verify_file_digest(&path, &wrong).await?);
    }

    tokio::fs::remove_file(path).await?;
    Ok(())
}

#[tokio::test]
async fn returns_prepare_failed_when_the_file_cannot_be_opened() {
    let preparer = ArtifactPreparer::new();
    let digest =
        Digest::new(DigestAlgorithm::Sha256, "0".repeat(64)).expect("测试摘要的格式应合法");

    let error = preparer
        .verify_file_digest(&temporary_path(), &digest)
        .await
        .expect_err("缺失文件必须返回准备失败");

    assert!(matches!(error, ArtifactPrepareError::PrepareFailed(_)));
}

fn temporary_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "burncloud-artifact-digest-{}.bin",
        std::process::id()
    ))
}
