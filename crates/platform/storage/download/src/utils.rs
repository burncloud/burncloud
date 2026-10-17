use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Component, Path, PathBuf};

use tokio::net::lookup_host;
use url::Url;

use crate::error::{DownloadError, Result};

/// 从下载 URL 中提取仓库名，作为下载文件名。
pub(super) fn extract_filename_from_url(url: &str) -> Option<String> {
    let path = url.split('?').next()?;
    let before_action = path
        .split_once("/resolve/")
        .map(|(before, _)| before)
        .or_else(|| path.strip_suffix("/repo"))?;

    before_action
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .map(str::to_string)
}

pub(super) async fn validate_url(url: &str) -> Result<()> {
    let parsed = Url::parse(url).map_err(|error| DownloadError::InvalidUrl(error.to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(DownloadError::InvalidUrl(
            "仅允许 HTTP 或 HTTPS".to_string(),
        ));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| DownloadError::InvalidUrl("URL 缺少主机名".to_string()))?;
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| DownloadError::InvalidUrl("URL 缺少有效端口".to_string()))?;

    let addresses = match host.parse::<IpAddr>() {
        Ok(address) => vec![address],
        Err(_) => lookup_host((host, port))
            .await
            .map_err(|error| DownloadError::InvalidUrl(format!("DNS 查询失败：{error}")))?
            .map(|address| address.ip())
            .collect(),
    };
    if addresses.is_empty() || addresses.iter().any(is_non_public_address) {
        return Err(DownloadError::InvalidUrl(
            "目标地址不能是回环、私有或链路本地地址".to_string(),
        ));
    }
    Ok(())
}

pub(super) async fn normalize_download_dir(dir: &str) -> Result<PathBuf> {
    let executable = std::env::current_exe()
        .map_err(|error| DownloadError::InvalidDirectory(error.to_string()))?;
    let executable_parent = executable
        .parent()
        .ok_or_else(|| DownloadError::InvalidDirectory("可执行文件没有父目录".to_string()))?;
    let executable_dir = tokio::fs::canonicalize(executable_parent)
        .await
        .map_err(|error| DownloadError::InvalidDirectory(error.to_string()))?;
    let models_dir = executable_dir.join("models");
    tokio::fs::create_dir_all(&models_dir)
        .await
        .map_err(|error| DownloadError::InvalidDirectory(error.to_string()))?;
    let models_dir = tokio::fs::canonicalize(models_dir)
        .await
        .map_err(|error| DownloadError::InvalidDirectory(error.to_string()))?;

    let requested = Path::new(dir);
    let starts_at_models = requested
        .components()
        .next()
        .is_some_and(|component| component.as_os_str() == "models");
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else if starts_at_models {
        executable_dir.join(requested)
    } else {
        models_dir.join(requested)
    };
    if requested
        .components()
        .any(|component| matches!(component, Component::ParentDir))
        || !candidate.starts_with(&models_dir)
    {
        return Err(DownloadError::InvalidDirectory(
            "目录必须位于可执行文件同级 models 子目录内".to_string(),
        ));
    }
    ensure_existing_path_inside(&models_dir, &candidate).await?;

    tokio::fs::create_dir_all(&candidate)
        .await
        .map_err(|error| DownloadError::InvalidDirectory(error.to_string()))?;
    let candidate = tokio::fs::canonicalize(candidate)
        .await
        .map_err(|error| DownloadError::InvalidDirectory(error.to_string()))?;
    if !candidate.starts_with(&models_dir) {
        return Err(DownloadError::InvalidDirectory(
            "目录解析后超出可执行文件同级 models 子目录".to_string(),
        ));
    }
    Ok(candidate)
}

pub(super) async fn existing_database_file(final_path: &str) -> Result<Option<PathBuf>> {
    let path = PathBuf::from(final_path);
    if !path.is_absolute() {
        return Ok(None);
    }

    match tokio::fs::metadata(&path).await {
        Ok(metadata) if metadata.is_file() => Ok(Some(path)),
        Ok(_) => Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(DownloadError::Failed(format!(
            "检查数据库下载文件失败：{error}"
        ))),
    }
}

async fn ensure_existing_path_inside(root: &Path, candidate: &Path) -> Result<()> {
    let relative = candidate.strip_prefix(root).map_err(|_| {
        DownloadError::InvalidDirectory("目录必须位于可执行文件同级 models 子目录内".to_string())
    })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        match component {
            Component::Normal(name) => current.push(name),
            Component::CurDir => continue,
            _ => {
                return Err(DownloadError::InvalidDirectory(
                    "下载目录包含无效路径组件".to_string(),
                ));
            }
        }
        match tokio::fs::symlink_metadata(&current).await {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                let resolved = tokio::fs::canonicalize(&current)
                    .await
                    .map_err(|error| DownloadError::InvalidDirectory(error.to_string()))?;
                if !resolved.starts_with(root) {
                    return Err(DownloadError::InvalidDirectory(
                        "目录符号链接指向可执行文件同级 models 子目录之外".to_string(),
                    ));
                }
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(DownloadError::InvalidDirectory(error.to_string()));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/support/database_file_path.rs"]
mod database_file_path_tests;

#[cfg(test)]
#[path = "../tests/support/dir_norm.rs"]
mod dir_norm_tests;

pub(super) fn sanitize_filename(filename: &str) -> Result<String> {
    let sanitized = filename
        .chars()
        .filter(|character| {
            !character.is_control()
                && !matches!(
                    character,
                    '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                )
        })
        .collect::<String>();
    let sanitized = sanitized.trim_end_matches([' ', '.']);
    if sanitized.is_empty() || matches!(sanitized, "." | "..") {
        return Err(DownloadError::InvalidFilename);
    }

    let stem = sanitized.split('.').next().unwrap_or_default();
    let upper_stem = stem.to_ascii_uppercase();
    let reserved = matches!(
        upper_stem.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );
    if reserved {
        return Ok(format!("_{sanitized}"));
    }
    Ok(sanitized.to_string())
}

fn is_non_public_address(address: &IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_non_public_ipv4(*address),
        IpAddr::V6(address) => is_non_public_ipv6(*address),
    }
}

fn is_non_public_ipv4(address: Ipv4Addr) -> bool {
    let [a, b, c, _] = address.octets();
    address.is_private()
        || address.is_loopback()
        || address.is_link_local()
        || address.is_broadcast()
        || address.is_unspecified()
        || address.is_multicast()
        || a == 0
        || (a == 100 && (64..=127).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 192 && b == 0 && c == 2)
        || (a == 198 && (b == 18 || b == 19))
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113)
        || a >= 240
}

fn is_non_public_ipv6(address: Ipv6Addr) -> bool {
    if let Some(ipv4) = address.to_ipv4_mapped() {
        return is_non_public_ipv4(ipv4);
    }
    let segments = address.segments();
    address.is_loopback()
        || address.is_unspecified()
        || address.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
}

#[cfg(test)]
#[path = "../tests/support/url_utils.rs"]
mod url_utils_tests;
