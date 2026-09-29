use std::path::{Path, PathBuf};
use std::str::FromStr;

use url::Url;

use super::artifact_source_err::artifactSourceErr;

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
    pub fn parse(value: impl AsRef<str>) -> Result<Self, artifactSourceErr> {
        value.as_ref().parse()
    }

    /// 返回文件来源中的路径；HTTP 来源返回 `None`。
    #[allow(non_snake_case)]
    pub fn as_filePath(&self) -> Option<&Path> {
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
    #[allow(non_snake_case)]
    pub fn is_filePath(&self) -> bool {
        matches!(self, Self::File(_))
    }

    /// 判断来源是否为 HTTP(S) URL。
    pub fn is_http(&self) -> bool {
        matches!(self, Self::Http(_))
    }

    /// 解析 URL，并根据协议决定来源类型。
    fn parse_url(value: &str) -> Result<Self, artifactSourceErr> {
        let url =
            Url::parse(value).map_err(|error| artifactSourceErr::InvalidUrl(error.to_string()))?;
        let scheme = url.scheme();

        match scheme.to_ascii_lowercase().as_str() {
            "http" | "https" => Ok(Self::Http(url)),
            "file" => url
                .to_file_path()
                .map(Self::File)
                .map_err(|_| artifactSourceErr::InvalidFileUrl(value.to_owned())),
            other => Err(artifactSourceErr::UnsupportedScheme(other.to_owned())),
        }
    }

    /// 验证本地路径不包含当前平台无法处理的空字节。
    fn parse_file_path(value: &str) -> Result<Self, artifactSourceErr> {
        match value.contains('\0') {
            true => Err(artifactSourceErr::InvalidFilePath(value.to_owned())),
            false => match Path::new(value).to_str() {
                Some(_) => Ok(Self::File(PathBuf::from(value))),
                None => Err(artifactSourceErr::InvalidFilePath(value.to_owned())),
            },
        }
    }

    /// 识别 Windows 驱动器路径，避免 URL 解析器把驱动器字母当作协议。
    fn is_windows_drive_path(value: &str) -> bool {
        let bytes = value.as_bytes();
        bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\')
    }
}

impl FromStr for ArtifactSource {
    type Err = artifactSourceErr;

    /// 按协议或当前操作系统的文件路径规则解析来源。
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.trim().is_empty() {
            return Err(artifactSourceErr::Empty);
        }

        match (cfg!(windows), Self::is_windows_drive_path(value)) {
            (true, true) => return Self::parse_file_path(value),
            _ => {}
        }

        let has_url_authority = value.contains("://");
        let parsed_url = Url::parse(value);

        match (has_url_authority, parsed_url) {
            (true, Ok(_)) => Self::parse_url(value),
            (true, Err(error)) => Err(artifactSourceErr::InvalidUrl(error.to_string())),
            (false, Ok(url)) if !url.scheme().is_empty() => Err(
                artifactSourceErr::UnsupportedScheme(url.scheme().to_owned()),
            ),
            (false, _) => Self::parse_file_path(value),
        }
    }
}

impl std::fmt::Display for ArtifactSource {
    /// 使用 URL 或文件管理器路径格式输出来源。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File(path) => path.display().fmt(formatter),
            Self::Http(url) => url.fmt(formatter),
        }
    }
}

impl From<&Path> for ArtifactSource {
    /// 将已经由调用方提供的路径转换为文件来源。
    fn from(value: &Path) -> Self {
        Self::File(value.to_path_buf())
    }
}

impl From<&Url> for ArtifactSource {
    /// 将 URL 转换为来源，并在协议不受支持时显式触发错误而不静默降级。
    fn from(value: &Url) -> Self {
        match Self::parse(value.as_str()) {
            Ok(source) => source,
            Err(error) => panic!("invalid artifact source URL: {error}"),
        }
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
        assert!(source.is_filePath());
        assert_eq!(
            source.as_filePath().unwrap(),
            Path::new("models/model.gguf")
        );
    }

    #[cfg(windows)]
    #[test]
    fn parses_windows_drive_paths_as_file_sources() {
        let source = ArtifactSource::parse(r"C:\models\model.gguf").unwrap();
        assert!(source.is_filePath());
    }

    #[cfg(unix)]
    #[test]
    fn parses_posix_absolute_paths_as_file_sources() {
        let source = ArtifactSource::parse("/models/model.gguf").unwrap();
        assert!(source.is_filePath());
    }

    #[test]
    fn parses_file_urls_for_the_deployed_platform() {
        let source = if cfg!(windows) {
            ArtifactSource::parse("file:///C:/models/model.gguf").unwrap()
        } else {
            ArtifactSource::parse("file:///models/model.gguf").unwrap()
        };
        assert!(source.is_filePath());
    }

    #[test]
    fn rejects_unsupported_schemes() {
        let error = ArtifactSource::parse("oci://registry.example/model:latest").unwrap_err();
        assert!(matches!(error, artifactSourceErr::UnsupportedScheme(_)));
    }

    #[test]
    fn rejects_empty_values() {
        assert_eq!(ArtifactSource::parse("   "), Err(artifactSourceErr::Empty));
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
