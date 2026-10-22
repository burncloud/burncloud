use std::path::PathBuf;

use burncloud_node_runtime::{
    ArtifactPreparer, ArtifactVerificationStatus, Digest, DigestAlgorithm,
};
use url::Url;

const L1_URL: &str =
    "https://modelscope.cn/models/Qwen/Qwen3-0.6B/resolve/master/model.safetensors";
const L1_SHA256: &str = "f47f71177f32bcd101b7573ec9171e6a57f4f4d31148d38e382306f42996874b";
const L1_FILE: &str = "Qwen3-0.6B";
const L1_SIZE: u64 = 1_503_300_328;

#[tokio::test]
async fn d006_interrupt_helper() -> Result<(), Box<dyn std::error::Error>> {
    let phase = std::env::var("BURNCLOUD_D006_PHASE")?;
    if phase == "first" {
        remove_download_artifacts(L1_FILE).await?;
        let preparer = ArtifactPreparer::new();
        let artifact = preparer
            .prepare(Url::parse(L1_URL)?, sha256_digest(L1_SHA256)?)
            .await?;
        return Err(format!(
            "D-006 first phase completed before interruption: {}",
            artifact.local_path().display()
        )
        .into());
    }

    if phase != "second" {
        return Err("BURNCLOUD_D006_PHASE must be first or second".into());
    }

    let preparer = ArtifactPreparer::new();
    let artifact = preparer
        .prepare(Url::parse(L1_URL)?, sha256_digest(L1_SHA256)?)
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
        L1_SIZE
    );

    drop(preparer);
    remove_download_artifacts(L1_FILE).await?;
    Ok(())
}

fn sha256_digest(value: &str) -> Result<Digest, burncloud_node_runtime::DigestError> {
    Digest::new(DigestAlgorithm::Sha256, value)
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
