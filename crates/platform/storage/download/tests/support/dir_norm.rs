#![allow(
    clippy::panic_in_result_fn,
    reason = "测试用 Result 传播文件操作错误，并用断言验证路径行为"
)]

#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)]
use std::os::windows::ffi::{OsStrExt, OsStringExt};
#[cfg(windows)]
use std::path::{Path, PathBuf};

use super::normalize_download_dir;

#[cfg(windows)]
fn regular_windows_path(path: &Path) -> PathBuf {
    let path_units: Vec<u16> = path.as_os_str().encode_wide().collect();
    let extended_prefix: Vec<u16> = r"\\?\".encode_utf16().collect();
    let slash_extended_prefix: Vec<u16> = r"//?/".encode_utf16().collect();
    let path_units = if path_units.starts_with(&extended_prefix) {
        &path_units[extended_prefix.len()..]
    } else if path_units.starts_with(&slash_extended_prefix) {
        &path_units[slash_extended_prefix.len()..]
    } else {
        return path.to_path_buf();
    };

    let unc_prefix: Vec<u16> = r"UNC\".encode_utf16().collect();
    let slash_unc_prefix: Vec<u16> = r"UNC/".encode_utf16().collect();
    let (path_units, mut regular_units) = if path_units.starts_with(&unc_prefix) {
        (
            &path_units[unc_prefix.len()..],
            r"\\".encode_utf16().collect(),
        )
    } else if path_units.starts_with(&slash_unc_prefix) {
        (
            &path_units[slash_unc_prefix.len()..],
            r"\\".encode_utf16().collect(),
        )
    } else {
        (path_units, Vec::new())
    };
    regular_units.extend(path_units.iter().map(|unit| {
        if *unit == b'/' as u16 {
            b'\\' as u16
        } else {
            *unit
        }
    }));
    PathBuf::from(OsString::from_wide(&regular_units))
}

#[cfg(windows)]
fn assert_regular_windows_path(path: &Path) {
    assert!(path.is_absolute());
    assert!(!path.to_string_lossy().starts_with(r"\\?\"));
}

#[tokio::test]
async fn resolves_download_directories_relative_to_the_executable(
) -> Result<(), Box<dyn std::error::Error>> {
    let executable = std::env::current_exe()?;
    let executable_dir =
        tokio::fs::canonicalize(executable.parent().ok_or("测试可执行文件没有父目录")?).await?;
    let models_dir = executable_dir.join("models");
    // 测试目录可能来自上一次失败运行；先清空它，避免残留的 models 目录影响真实部署测试。
    if tokio::fs::try_exists(&models_dir).await? {
        tokio::fs::remove_dir_all(&models_dir).await?;
    }
    let nested_dir = models_dir.join("sub");
    let unique_dir = models_dir.join(format!("dir-normalization-{}", std::process::id()));
    let outside_dir = executable_dir.join("dir-normalization-outside");
    #[cfg(windows)]
    let unique_dir_input = regular_windows_path(&unique_dir);
    #[cfg(not(windows))]
    let unique_dir_input = unique_dir.clone();
    #[cfg(windows)]
    let outside_dir_input = regular_windows_path(&outside_dir);
    #[cfg(not(windows))]
    let outside_dir_input = outside_dir.clone();

    let test_result = async {
        let normalized_models = normalize_download_dir("models").await?;
        #[cfg(not(windows))]
        assert_eq!(
            normalized_models,
            tokio::fs::canonicalize(&models_dir).await?
        );
        #[cfg(windows)]
        {
            assert_regular_windows_path(&normalized_models);
            assert_eq!(
                tokio::fs::canonicalize(&normalized_models).await?,
                tokio::fs::canonicalize(&models_dir).await?
            );
        }

        #[cfg(windows)]
        let absolute_models_input = regular_windows_path(&models_dir);
        #[cfg(not(windows))]
        let absolute_models_input = models_dir.clone();
        let normalized_absolute_models = normalize_download_dir(
            absolute_models_input
                .to_str()
                .ok_or("测试 models 绝对路径不是有效 UTF-8")?,
        )
        .await?;
        #[cfg(not(windows))]
        assert_eq!(
            normalized_absolute_models,
            tokio::fs::canonicalize(&models_dir).await?
        );
        #[cfg(windows)]
        {
            assert_regular_windows_path(&normalized_absolute_models);
            assert_eq!(
                tokio::fs::canonicalize(&normalized_absolute_models).await?,
                tokio::fs::canonicalize(&models_dir).await?
            );
        }

        let normalized_nested = normalize_download_dir("models/sub").await?;
        #[cfg(not(windows))]
        assert_eq!(
            normalized_nested,
            tokio::fs::canonicalize(&nested_dir).await?
        );
        #[cfg(windows)]
        {
            assert_regular_windows_path(&normalized_nested);
            assert_eq!(
                tokio::fs::canonicalize(&normalized_nested).await?,
                tokio::fs::canonicalize(&nested_dir).await?
            );
        }

        let normalized_absolute = normalize_download_dir(
            unique_dir_input
                .to_str()
                .ok_or("测试绝对路径不是有效 UTF-8")?,
        )
        .await?;
        #[cfg(not(windows))]
        assert_eq!(
            normalized_absolute,
            tokio::fs::canonicalize(&unique_dir).await?
        );
        #[cfg(windows)]
        {
            assert_regular_windows_path(&normalized_absolute);
            assert_eq!(
                tokio::fs::canonicalize(&normalized_absolute).await?,
                tokio::fs::canonicalize(&unique_dir).await?
            );
        }

        assert!(normalize_download_dir("../x").await.is_err());
        assert!(normalize_download_dir(
            outside_dir_input
                .to_str()
                .ok_or("测试外部路径不是有效 UTF-8")?,
        )
        .await
        .is_err());

        let current_dir_models = std::env::current_dir()?.join("models");
        if let Ok(current_dir_models) = tokio::fs::canonicalize(current_dir_models).await {
            assert_ne!(
                normalized_models, current_dir_models,
                "下载目录不能以当前工作目录为基准"
            );
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    // 即使断言失败，也必须清理本测试创建的目录，避免污染后续真实部署测试。
    if tokio::fs::try_exists(&models_dir).await? {
        tokio::fs::remove_dir_all(&models_dir).await?;
    }

    test_result
}
