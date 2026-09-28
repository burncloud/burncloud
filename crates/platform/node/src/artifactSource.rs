// 简单实现，待修改
use std::path::{Path, PathBuf};

use url::Url;

/// 工件来源类型，避免调用方以未约束字符串传递来源信息。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArtifactSource {
    File(PathBuf),
    Http(Url),
    Oci(String),
}

impl ArtifactSource {
    /// 按来源格式解析字符串：HTTP(S) 使用 URL，本地绝对路径使用文件来源，
    /// 其余值作为 OCI 引用保留。
    pub fn parse(value: impl AsRef<str>) -> Result<Self, url::ParseError> {
        let value = value.as_ref();
        if value.starts_with("http://") || value.starts_with("https://") {
            return Url::parse(value).map(Self::Http);
        }
        if Path::new(value).is_absolute() {
            return Ok(Self::File(PathBuf::from(value)));
        }
        Ok(Self::Oci(value.to_owned()))
    }
}

impl From<PathBuf> for ArtifactSource {
    fn from(value: PathBuf) -> Self {
        Self::File(value)
    }
}

impl From<Url> for ArtifactSource {
    fn from(value: Url) -> Self {
        Self::Http(value)
    }
}

impl From<String> for ArtifactSource {
    fn from(value: String) -> Self {
        Self::parse(&value).unwrap_or(Self::Oci(value))
    }
}

impl From<&str> for ArtifactSource {
    fn from(value: &str) -> Self {
        Self::parse(value).unwrap_or_else(|_| Self::Oci(value.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_http_urls_as_http_sources() {
        let source = ArtifactSource::parse("https://example.com/model.gguf").unwrap();
        assert!(matches!(source, ArtifactSource::Http(_)));
    }

    #[test]
    fn parses_absolute_paths_as_file_sources() {
        let source = ArtifactSource::parse("C:\\models\\model.gguf").unwrap();
        assert!(matches!(source, ArtifactSource::File(_)));
    }

    #[test]
    fn parses_other_references_as_oci_sources() {
        let source = ArtifactSource::parse("registry.example/model:latest").unwrap();
        assert_eq!(
            source,
            ArtifactSource::Oci("registry.example/model:latest".into())
        );
    }
}
