use std::path::{Path, PathBuf};
use std::str::FromStr;

use url::Url;

use super::artifact_source_error::ArtifactSourceError;

/// 工件来源类型，只允许本地文件路径或 HTTP(S) URL。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArtifactSource {
    /// 当前操作系统文件管理器使用的路径。
    File(PathBuf),
    /// 使用 HTTP 或 HTTPS 协议的远程地址。
    Http(Url),
}

impl ArtifactSource {
    /// 将字符串解析为文件路径或 HTTP(S) URL。
    pub fn parse(value: impl AsRef<str>) -> Result<Self, ArtifactSourceError> {
        value.as_ref().parse()
    }

    /// 返回文件来源中的路径；HTTP 来源返回 `None`。
    pub fn as_file_path(&self) -> Option<&Path> {
        match self {
            Self::File(path) => Some(path.as_path()),
            Self::Http(_) => None,
        }
    }

    /// 返回 HTTP(S) 来源中的 URL；文件来源返回 `None`。
    pub fn as_http(&self) -> Option<&Url> {
        match self {
            Self::File(_) => None,
            Self::Http(url) => Some(url),
        }
    }

    /// 判断来源是否为文件路径。
    pub fn is_file_path(&self) -> bool {
        matches!(self, Self::File(_))
    }

    /// 判断来源是否为 HTTP(S) URL。
    pub fn is_http(&self) -> bool {
        matches!(self, Self::Http(_))
    }

    /// 解析 URL，并根据协议决定来源类型。
    fn parse_url(value: &str) -> Result<Self, ArtifactSourceError> {
        // 先解析 URL，再根据协议白名单转换为支持的来源类型。
        let url = Url::parse(value)
            .map_err(|error| ArtifactSourceError::InvalidUrl(error.to_string()))?;
        let scheme = url.scheme();

        match scheme.to_ascii_lowercase().as_str() {
            "http" | "https" => Ok(Self::Http(url)),
            "file" => url
                .to_file_path()
                .map(Self::File)
                .map_err(|_| ArtifactSourceError::InvalidFileUrl(value.to_owned())),
            other => Err(ArtifactSourceError::UnsupportedScheme(other.to_owned())),
        }
    }

    /// 验证本地路径不包含当前平台无法处理的空字节。
    fn parse_file_path(value: &str) -> Result<Self, ArtifactSourceError> {
        // 空字节无法出现在有效的本地路径中，必须在创建 PathBuf 前拒绝。
        match value.contains('\0') {
            true => Err(ArtifactSourceError::InvalidFilePath(value.to_owned())),
            false => match Path::new(value).to_str() {
                Some(_) => Ok(Self::File(PathBuf::from(value))),
                None => Err(ArtifactSourceError::InvalidFilePath(value.to_owned())),
            },
        }
    }

    /// 识别 Windows 驱动器路径，避免 URL 解析器把驱动器字母当作协议。
    fn is_windows_drive_path(value: &str) -> bool {
        // Windows 驱动器路径包含盘符、冒号和路径分隔符三个特征。
        let bytes = value.as_bytes();
        bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\')
    }
}

impl FromStr for ArtifactSource {
    type Err = ArtifactSourceError;

    /// 按协议或当前操作系统的文件路径规则解析来源。
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // 空白输入没有可用来源，先于 URL 和路径分支进行校验。
        if value.trim().is_empty() {
            return Err(ArtifactSourceError::Empty);
        }

        if let (true, true) = (cfg!(windows), Self::is_windows_drive_path(value)) {
            return Self::parse_file_path(value);
        }

        let has_url_authority = value.contains("://");
        let parsed_url = Url::parse(value);

        // 带协议分隔符的输入必须是有效 URL；其他输入按路径或协议名处理。
        match (has_url_authority, parsed_url) {
            (true, Ok(_)) => Self::parse_url(value),
            (true, Err(error)) => Err(ArtifactSourceError::InvalidUrl(error.to_string())),
            (false, Ok(url)) if !url.scheme().is_empty() => Err(
                ArtifactSourceError::UnsupportedScheme(url.scheme().to_owned()),
            ),
            (false, _) => Self::parse_file_path(value),
        }
    }
}

impl std::fmt::Display for ArtifactSource {
    /// 使用 URL 或文件管理器路径格式输出来源。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 文件来源使用平台路径格式，远程来源保留 URL 格式。
        match self {
            Self::File(path) => path.display().fmt(formatter),
            Self::Http(url) => url.fmt(formatter),
        }
    }
}

impl From<&Path> for ArtifactSource {
    /// 将已经由调用方提供的路径转换为文件来源。
    fn from(value: &Path) -> Self {
        // 复制路径所有权，保证来源值独立于调用方的路径引用。
        Self::File(value.to_path_buf())
    }
}

impl TryFrom<&Url> for ArtifactSource {
    type Error = ArtifactSourceError;

    /// 将 URL 转换为来源，并使用 `ArtifactSourceError` 返回协议校验错误。
    fn try_from(value: &Url) -> Result<Self, Self::Error> {
        // 复用统一 URL 解析逻辑，确保字符串和 URL 入口的校验一致。
        Self::parse_url(value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_http_urls_case_insensitively() {
        let source = ArtifactSource::parse("HTTP://example.com/model.gguf").unwrap();
        assert!(source.is_http());
        assert_eq!(source.as_http().unwrap().scheme(), "http");
    }

    #[test]
    fn parses_relative_paths_as_file_sources() {
        let source = ArtifactSource::parse("models/model.gguf").unwrap();
        assert!(source.is_file_path());
        assert_eq!(
            source.as_file_path().unwrap(),
            Path::new("models/model.gguf")
        );
    }

    #[cfg(windows)]
    #[test]
    fn parses_windows_drive_paths_as_file_sources() {
        let source = ArtifactSource::parse(r"C:\models\model.gguf").unwrap();
        assert!(source.is_file_path());
    }

    #[cfg(unix)]
    #[test]
    fn parses_posix_absolute_paths_as_file_sources() {
        let source = ArtifactSource::parse("/models/model.gguf").unwrap();
        assert!(source.is_file_path());
    }

    #[test]
    fn parses_file_urls_for_the_deployed_platform() {
        let source = if cfg!(windows) {
            ArtifactSource::parse("file:///C:/models/model.gguf").unwrap()
        } else {
            ArtifactSource::parse("file:///models/model.gguf").unwrap()
        };
        assert!(source.is_file_path());
    }

    #[test]
    fn rejects_unsupported_schemes() {
        let error = ArtifactSource::parse("oci://registry.example/model:latest").unwrap_err();
        assert!(matches!(error, ArtifactSourceError::UnsupportedScheme(_)));
    }

    #[test]
    fn try_from_url_returns_artifact_source_error() {
        let url = Url::parse("ftp://example.com/model.gguf").unwrap();
        let error = ArtifactSource::try_from(&url).unwrap_err();
        assert_eq!(
            error,
            ArtifactSourceError::UnsupportedScheme("ftp".to_owned())
        );
    }

    #[test]
    fn rejects_empty_values() {
        assert_eq!(
            ArtifactSource::parse("   "),
            Err(ArtifactSourceError::Empty)
        );
    }

    #[test]
    fn displays_sources() {
        assert_eq!(
            ArtifactSource::parse("models/model.gguf")
                .unwrap()
                .to_string(),
            "models/model.gguf"
        );
        assert_eq!(
            ArtifactSource::parse("https://example.com/model.gguf")
                .unwrap()
                .to_string(),
            "https://example.com/model.gguf"
        );
    }
}
