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

#[cfg_attr(
    not(windows),
    ignore = "D-006 中断恢复测试需要 Windows aria2 可执行文件"
)]
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
    if artifact.verification_status() != ArtifactVerificationStatus::Verified {
        return Err("D-006 恢复后的工件未通过摘要校验".into());
    }
    if !artifact.local_path().is_absolute() {
        return Err("D-006 恢复后的工件路径不是绝对路径".into());
    }
    if !artifact.local_path().is_file() {
        return Err("D-006 恢复后的工件文件不存在".into());
    }
    if !artifact.local_path().starts_with(models_directory()?) {
        return Err("D-006 恢复后的工件路径不在 models 目录内".into());
    }
    if tokio::fs::metadata(artifact.local_path()).await?.len() != L1_SIZE {
        return Err("D-006 恢复后的工件大小不符合预期".into());
    }

    drop(preparer);
    remove_download_artifacts(L1_FILE).await?;
    Ok(())
}

fn sha256_digest(value: &str) -> Result<Digest, burncloud_node_runtime::DigestError> {
    Digest::new(DigestAlgorithm::Sha256, value)
}

fn models_directory() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let executable = std::env::current_exe()?;
    let parent = executable.parent().ok_or("测试可执行文件应有父目录")?;
    Ok(parent.join("models"))
}

async fn remove_download_artifacts(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let models = models_directory()?;
    for path in [models.join(name), models.join(format!("{name}.aria2"))] {
        match tokio::fs::remove_file(path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Box::new(error)),
        }
    }
    Ok(())
}
