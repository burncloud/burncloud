#![allow(
    clippy::panic_in_result_fn,
    reason = "测试用 Result 传播文件操作错误，并用断言验证路径行为"
)]

use super::normalize_download_dir;

#[tokio::test]
async fn resolves_download_directories_relative_to_the_executable(
) -> Result<(), Box<dyn std::error::Error>> {
    let executable = std::env::current_exe()?;
    let executable_dir =
        tokio::fs::canonicalize(executable.parent().ok_or("测试可执行文件没有父目录")?).await?;
    let models_dir = executable_dir.join("models");
    let models_existed = tokio::fs::try_exists(&models_dir).await?;
    let nested_dir = models_dir.join("sub");
    let nested_existed = tokio::fs::try_exists(&nested_dir).await?;
    let unique_dir = models_dir.join(format!("dir-normalization-{}", std::process::id()));
    let outside_dir = executable_dir.join("dir-normalization-outside");

    let normalized_models = normalize_download_dir("models").await?;
    assert_eq!(
        normalized_models,
        tokio::fs::canonicalize(&models_dir).await?
    );

    let normalized_nested = normalize_download_dir("models/sub").await?;
    assert_eq!(
        normalized_nested,
        tokio::fs::canonicalize(&nested_dir).await?
    );

    let normalized_absolute =
        normalize_download_dir(unique_dir.to_str().ok_or("测试绝对路径不是有效 UTF-8")?).await?;
    assert_eq!(
        normalized_absolute,
        tokio::fs::canonicalize(&unique_dir).await?
    );

    assert!(normalize_download_dir("../x").await.is_err());
    assert!(
        normalize_download_dir(outside_dir.to_str().ok_or("测试外部路径不是有效 UTF-8")?,)
            .await
            .is_err()
    );

    tokio::fs::remove_dir_all(&unique_dir).await?;
    if !nested_existed {
        tokio::fs::remove_dir_all(&nested_dir).await?;
    }
    if !models_existed {
        let mut entries = tokio::fs::read_dir(&models_dir).await?;
        if entries.next_entry().await?.is_none() {
            tokio::fs::remove_dir(&models_dir).await?;
        }
    }

    let current_dir_models = std::env::current_dir()?.join("models");
    if let Ok(current_dir_models) = tokio::fs::canonicalize(current_dir_models).await {
        assert_ne!(
            normalized_models, current_dir_models,
            "下载目录不能以当前工作目录为基准"
        );
    }
    Ok(())
}
