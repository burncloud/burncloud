use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Component, Path, PathBuf};

use tokio::net::lookup_host;
use url::Url;

use crate::error::{DownloadError, Result};

/// 从 URL 文本中提取文件名。
pub(super) fn extract_filename_from_url(url: &str) -> Option<String> {
    let url_without_query = url.split('?').next()?;

    let filename = url_without_query.split('/').next_back()?;

    if filename.is_empty() {
        None
    } else {
        Some(filename.to_string())
    }
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
    let current_dir = std::env::current_dir()
        .map_err(|error| DownloadError::InvalidDirectory(error.to_string()))?;
    let project_dir = tokio::fs::canonicalize(current_dir)
        .await
        .map_err(|error| DownloadError::InvalidDirectory(error.to_string()))?;
    let downloads_dir = project_dir.join("downloads");
    tokio::fs::create_dir_all(&downloads_dir)
        .await
        .map_err(|error| DownloadError::InvalidDirectory(error.to_string()))?;
    let downloads_dir = tokio::fs::canonicalize(downloads_dir)
        .await
        .map_err(|error| DownloadError::InvalidDirectory(error.to_string()))?;
    if !downloads_dir.starts_with(&project_dir) {
        return Err(DownloadError::InvalidDirectory(
            "项目 downloads 目录不能指向项目目录之外".to_string(),
        ));
    }

    let requested = Path::new(dir);
    let starts_at_downloads = requested
        .components()
        .next()
        .is_some_and(|component| component.as_os_str() == "downloads");
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else if starts_at_downloads {
        project_dir.join(requested)
    } else {
        downloads_dir.join(requested)
    };
    if requested
        .components()
        .any(|component| matches!(component, Component::ParentDir))
        || !candidate.starts_with(&downloads_dir)
    {
        return Err(DownloadError::InvalidDirectory(
            "目录必须位于项目 downloads 子目录内".to_string(),
        ));
    }
    ensure_existing_path_inside(&downloads_dir, &candidate).await?;

    tokio::fs::create_dir_all(&candidate)
        .await
        .map_err(|error| DownloadError::InvalidDirectory(error.to_string()))?;
    let candidate = tokio::fs::canonicalize(candidate)
        .await
        .map_err(|error| DownloadError::InvalidDirectory(error.to_string()))?;
    if !candidate.starts_with(&downloads_dir) {
        return Err(DownloadError::InvalidDirectory(
            "目录解析后超出项目 downloads 子目录".to_string(),
        ));
    }
    Ok(candidate)
}

async fn ensure_existing_path_inside(root: &Path, candidate: &Path) -> Result<()> {
    let relative = candidate.strip_prefix(root).map_err(|_| {
        DownloadError::InvalidDirectory("目录必须位于项目 downloads 子目录内".to_string())
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
                        "目录符号链接指向项目 downloads 子目录之外".to_string(),
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
#[allow(
    clippy::expect_used,
    reason = "测试地址均为固定有效字面量，expect 仅用于构造测试输入"
)]
mod tests {
    use super::{extract_filename_from_url, is_non_public_address, sanitize_filename};
    use std::net::IpAddr;

    #[test]
    fn extracts_filename_without_query() {
        assert_eq!(
            extract_filename_from_url("https://example.invalid/path/model.gguf?token=x").as_deref(),
            Some("model.gguf")
        );
        assert_eq!(
            extract_filename_from_url("https://example.invalid/path/"),
            None
        );
    }

    #[test]
    fn sanitizes_path_separators_and_windows_device_names() {
        assert!(matches!(
            sanitize_filename("../model\\file.gguf"),
            Ok(name) if name == "..modelfile.gguf"
        ));
        assert!(matches!(
            sanitize_filename("CON.txt"),
            Ok(name) if name == "_CON.txt"
        ));
        assert!(sanitize_filename("...").is_err());
    }

    #[test]
    fn rejects_non_public_ip_ranges() {
        for address in ["127.0.0.1", "10.1.2.3", "169.254.1.1", "::1", "fc00::1"] {
            let address = address.parse::<IpAddr>().expect("valid test address");
            assert!(is_non_public_address(&address), "{address}");
        }
        for address in ["8.8.8.8", "2606:4700:4700::1111"] {
            let address = address.parse::<IpAddr>().expect("valid test address");
            assert!(!is_non_public_address(&address), "{address}");
        }
    }
}
