use std::path::PathBuf;
use std::sync::Arc;

use burncloud_node_runtime::{
    ArtifactPreparer, ArtifactVerificationStatus, Digest, DigestAlgorithm,
};
use url::Url;

#[path = "support/artifact_preparer_test_database.rs"]
mod test_database;

const S1_URL: &str =
    "https://modelscope.cn/models/AI-ModelScope/bge-small-zh-v1.5/resolve/master/model.safetensors";
const S1_SHA256: &str = "354763b9b1357bc9c44f62c6be2276321081ed2567773608c0d0785b61d5a026";
const S1_FILE: &str = "bge-small-zh-v1.5";
const S1_SIZE: u64 = 95_827_648;

const S2_URL: &str =
    "https://modelscope.cn/models/AI-ModelScope/bge-small-en-v1.5/resolve/master/model.safetensors";
const S2_FILE: &str = "bge-small-en-v1.5";

const S3_URL: &str =
    "https://modelscope.cn/models/intfloat/e5-small-v2/resolve/master/model.safetensors";
const S3_SHA256: &str = "45bfa60070649aae2244fbc9d508537779b93b6f353c17b0f95ceccb1c5116c1";
const S3_FILE: &str = "e5-small-v2";
const S3_SIZE: u64 = 133_466_304;

const S4_URL: &str =
    "https://modelscope.cn/models/AI-ModelScope/all-MiniLM-L6-v2/resolve/master/model.safetensors";
const S4_SHA256: &str = "53aa51172d142c89d9012cce15ae4d6cc0ca6895895114379cacb4fab128d9db";
const S4_FILE: &str = "all-MiniLM-L6-v2";
const S4_SIZE: u64 = 90_868_376;

#[tokio::test]
async fn d001_small_file_is_prepared_and_verified() -> Result<(), Box<dyn std::error::Error>> {
    let _database = test_database::isolated_database("d001")?;
    remove_download_artifacts(S1_FILE).await?;

    let preparer = ArtifactPreparer::new();
    let artifact = preparer
        .prepare(Url::parse(S1_URL)?, sha256_digest(S1_SHA256)?)
        .await?;

    assert_eq!(
        artifact.verification_status(),
        ArtifactVerificationStatus::Verified
    );
    assert!(artifact.local_path().is_absolute());
    assert!(artifact.local_path().is_file());
    assert!(artifact.local_path().starts_with(models_directory()));
    assert_eq!(
        tokio::fs::metadata(artifact.local_path()).await?.len(),
        S1_SIZE
    );

    drop(preparer);
    _database.cleanup().await?;
    remove_download_artifacts(S1_FILE).await?;
    Ok(())
}

#[tokio::test]
async fn d002_mismatched_digest_returns_failed_receipt_and_allows_recovery(
) -> Result<(), Box<dyn std::error::Error>> {
    let _database = test_database::isolated_database("d002")?;
    remove_download_artifacts(S1_FILE).await?;
    remove_download_artifacts(S2_FILE).await?;

    let preparer = ArtifactPreparer::new();
    let failed = preparer
        .prepare(
            Url::parse(S2_URL)?,
            Digest::new(DigestAlgorithm::Sha256, "0".repeat(64))?,
        )
        .await?;

    assert_eq!(
        failed.verification_status(),
        ArtifactVerificationStatus::Failed
    );
    assert!(failed.local_path().as_os_str().is_empty());
    assert!(!models_directory().join(S2_FILE).exists());
    assert!(!models_directory().join(format!("{S2_FILE}.aria2")).exists());

    let recovered = preparer
        .prepare(Url::parse(S1_URL)?, sha256_digest(S1_SHA256)?)
        .await?;
    assert!(recovered.is_verified());
    assert_eq!(recovered.local_path().metadata()?.len(), S1_SIZE);

    drop(preparer);
    _database.cleanup().await?;
    remove_download_artifacts(S1_FILE).await?;
    remove_download_artifacts(S2_FILE).await?;
    Ok(())
}

#[tokio::test]
async fn d003_same_resource_concurrent_requests_share_one_result(
) -> Result<(), Box<dyn std::error::Error>> {
    let _database = test_database::isolated_database("d003")?;
    remove_download_artifacts(S3_FILE).await?;

    let preparer = Arc::new(ArtifactPreparer::new());
    let first = prepare_with(&preparer, S3_URL, S3_SHA256);
    let second = prepare_with(&preparer, S3_URL, S3_SHA256);
    let (first, second) = tokio::join!(first, second);
    let first = first?;
    let second = second?;

    assert!(first.is_verified());
    assert!(second.is_verified());
    assert_eq!(first.local_path(), second.local_path());
    assert_eq!(
        tokio::fs::metadata(first.local_path()).await?.len(),
        S3_SIZE
    );

    drop(preparer);
    _database.cleanup().await?;
    remove_download_artifacts(S3_FILE).await?;
    Ok(())
}

#[tokio::test]
async fn d004_different_resource_concurrent_requests_remain_independent(
) -> Result<(), Box<dyn std::error::Error>> {
    let _database = test_database::isolated_database("d004")?;
    remove_download_artifacts(S1_FILE).await?;
    remove_download_artifacts(S4_FILE).await?;

    let preparer = Arc::new(ArtifactPreparer::new());
    let first = prepare_with(&preparer, S1_URL, S1_SHA256);
    let second = prepare_with(&preparer, S4_URL, S4_SHA256);
    let (first, second) = tokio::join!(first, second);
    let first = first?;
    let second = second?;

    assert!(first.is_verified());
    assert!(second.is_verified());
    assert_ne!(first.local_path(), second.local_path());
    assert_eq!(
        tokio::fs::metadata(first.local_path()).await?.len(),
        S1_SIZE
    );
    assert_eq!(
        tokio::fs::metadata(second.local_path()).await?.len(),
        S4_SIZE
    );

    drop(preparer);
    _database.cleanup().await?;
    remove_download_artifacts(S1_FILE).await?;
    remove_download_artifacts(S4_FILE).await?;
    Ok(())
}

#[tokio::test]
async fn d005_source_and_missing_resource_errors_do_not_block_next_prepare(
) -> Result<(), Box<dyn std::error::Error>> {
    let _database = test_database::isolated_database("d005")?;
    remove_download_artifacts(S1_FILE).await?;
    let preparer = ArtifactPreparer::new();
    let digest = Digest::new(DigestAlgorithm::Sha256, "0".repeat(64))?;

    for url in [
        "https://example.com/models/example/resolve/main/model.safetensors",
        "ftp://modelscope.cn/models/example/resolve/main/model.safetensors",
    ] {
        assert!(preparer
            .prepare(Url::parse(url)?, digest.clone())
            .await
            .is_err());
    }

    let missing = preparer
        .prepare(
            Url::parse(
                "https://modelscope.cn/models/AI-ModelScope/bge-small-zh-v1.5/resolve/master/definitely-not-found.bin",
            )?,
            digest,
        )
        .await;
    assert!(missing.is_err());

    let recovered = preparer
        .prepare(Url::parse(S1_URL)?, sha256_digest(S1_SHA256)?)
        .await?;
    assert!(recovered.is_verified());
    assert!(recovered.local_path().starts_with(models_directory()));

    drop(preparer);
    _database.cleanup().await?;
    remove_download_artifacts(S1_FILE).await?;
    Ok(())
}

fn sha256_digest(value: &str) -> Result<Digest, burncloud_node_runtime::DigestError> {
    Digest::new(DigestAlgorithm::Sha256, value)
}

async fn prepare_with(
    preparer: &ArtifactPreparer,
    url: &str,
    digest: &str,
) -> Result<burncloud_node_runtime::PreparedArtifact, burncloud_node_runtime::ArtifactPrepareError>
{
    preparer
        .prepare(
            Url::parse(url).map_err(|error| {
                burncloud_node_runtime::ArtifactPrepareError::PrepareFailed(error.to_string())
            })?,
            sha256_digest(digest).map_err(|error| {
                burncloud_node_runtime::ArtifactPrepareError::PrepareFailed(error.to_string())
            })?,
        )
        .await
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
