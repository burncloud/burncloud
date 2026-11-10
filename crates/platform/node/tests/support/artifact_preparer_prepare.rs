#![allow(clippy::panic_in_result_fn, reason = "测试断言用于报告准备流程失败")]

use std::path::PathBuf;

use super::super::artifact_request::ArtifactRequest;
use super::ArtifactPreparer;
use crate::preparation::{
    ArtifactPrepareError, ArtifactSource, ArtifactVerificationStatus, Digest, DigestAlgorithm,
};
use url::Url;

const S1_URL: &str =
    "https://modelscope.cn/models/AI-ModelScope/bge-small-zh-v1.5/resolve/master/model.safetensors";
const S1_SHA256: &str = "354763b9b1357bc9c44f62c6be2276321081ed2567773608c0d0785b61d5a026";
const S1_FILE: &str = "bge-small-zh-v1.5";
const S2_URL: &str =
    "https://modelscope.cn/models/AI-ModelScope/bge-small-en-v1.5/resolve/master/model.safetensors";
const S2_FILE: &str = "bge-small-en-v1.5";

#[tokio::test]
async fn rejects_unsupported_sources_before_starting_the_download_manager() {
    let preparer = ArtifactPreparer::new();
    let digest =
        Digest::new(DigestAlgorithm::Sha256, "0".repeat(64)).expect("测试摘要的格式应合法");

    let error = preparer
        .prepare(
            Url::parse("https://example.com/models/example/resolve/main/model.gguf")
                .expect("测试 URL 应合法"),
            digest,
        )
        .await
        .expect_err("未知主机必须被拒绝");

    assert!(matches!(error, ArtifactPrepareError::PrepareFailed(_)));
    let lifecycle = preparer.lifecycle.lock().await;
    assert_eq!(lifecycle.ref_count, 0);
    assert!(lifecycle.download_manager.is_none());
}

#[cfg_attr(not(windows), ignore = "真实 aria2 下载测试需要 Windows 可执行文件")]
#[tokio::test]
async fn downloads_a_supported_artifact_into_models() -> Result<(), Box<dyn std::error::Error>> {
    let _database = super::test_database::isolated_database("download")?;
    remove_download_artifacts(S1_FILE).await?;
    let preparer = ArtifactPreparer::new();
    let source = ArtifactSource::parse(S1_URL)?;
    let request = ArtifactRequest::new(source, Digest::parse(&format!("sha256:{S1_SHA256}"))?);
    let manager = preparer.acquire_manager().await?;
    let result = preparer.download_artifact(&manager, &request).await;
    drop(manager);
    preparer.release_manager().await;
    let path = result?;

    assert!(path.is_absolute());
    assert!(path.starts_with(models_directory()));
    assert!(!path.as_os_str().is_empty());
    _database.cleanup().await?;
    remove_download_artifacts(S1_FILE).await?;
    Ok(())
}

#[cfg_attr(not(windows), ignore = "真实 aria2 下载测试需要 Windows 可执行文件")]
#[tokio::test]
async fn prepares_a_correctly_verified_artifact_once() -> Result<(), Box<dyn std::error::Error>> {
    let _database = super::test_database::isolated_database("verified")?;
    remove_download_artifacts(S1_FILE).await?;
    let preparer = ArtifactPreparer::new();
    let source = ArtifactSource::parse(S1_URL)?;
    let request = ArtifactRequest::new(source, Digest::parse(&format!("sha256:{S1_SHA256}"))?);
    let manager = preparer.acquire_manager().await?;
    let result = preparer.prepare_with_manager(&manager, &request).await;
    drop(manager);
    preparer.release_manager().await;
    let artifact = result?;

    assert_eq!(
        artifact.verification_status(),
        ArtifactVerificationStatus::Verified
    );
    assert!(artifact.is_verified());
    assert!(artifact.local_path().is_absolute());
    assert!(artifact.local_path().is_file());
    _database.cleanup().await?;
    remove_download_artifacts(S1_FILE).await?;
    Ok(())
}

#[cfg_attr(not(windows), ignore = "真实 aria2 下载测试需要 Windows 可执行文件")]
#[tokio::test]
async fn retries_once_and_returns_a_failed_empty_receipt_on_digest_mismatch(
) -> Result<(), Box<dyn std::error::Error>> {
    let _database = super::test_database::isolated_database("mismatch")?;
    remove_download_artifacts(S2_FILE).await?;
    let preparer = ArtifactPreparer::new();
    let source = ArtifactSource::parse(S2_URL)?;
    let request = ArtifactRequest::new(
        source,
        Digest::new(DigestAlgorithm::Sha256, "0".repeat(64))?,
    );
    let manager = preparer.acquire_manager().await?;
    let result = preparer.prepare_with_manager(&manager, &request).await;
    drop(manager);
    preparer.release_manager().await;
    let artifact = result?;

    assert_eq!(
        artifact.verification_status(),
        ArtifactVerificationStatus::Failed
    );
    assert!(artifact.local_path().as_os_str().is_empty());
    assert!(!models_directory().join(S2_FILE).exists());
    assert!(!models_directory().join(format!("{S2_FILE}.aria2")).exists());
    _database.cleanup().await?;
    remove_download_artifacts(S2_FILE).await?;
    Ok(())
}

fn models_directory() -> PathBuf {
    std::env::current_exe()
        .expect("测试可执行文件路径应可取得")
        .parent()
        .expect("测试可执行文件应有父目录")
        .join("models")
}

async fn remove_download_artifacts(name: &str) -> Result<(), std::io::Error> {
    for path in [
        models_directory().join(name),
        models_directory().join(format!("{name}.aria2")),
    ] {
        match tokio::fs::remove_file(path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}
