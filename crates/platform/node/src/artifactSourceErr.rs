use std::fmt;

/// `ArtifactSource` 解析失败时返回的错误类型。
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum artifactSourceErr {
    /// 输入为空或只包含空白字符。
    Empty,
    /// URL 文本无法被解析。
    InvalidUrl(String),
    /// URL 使用了不支持的协议。
    UnsupportedScheme(String),
    /// `file://` URL 无法转换为当前操作系统的文件路径。
    InvalidFileUrl(String),
    /// 文件路径包含当前平台不允许的内容。
    InvalidFilePath(String),
}

impl fmt::Display for artifactSourceErr {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("artifact source cannot be empty"),
            Self::InvalidUrl(value) => write!(formatter, "invalid URL: {value}"),
            Self::UnsupportedScheme(scheme) => {
                write!(formatter, "unsupported artifact source scheme: {scheme}")
            }
            Self::InvalidFileUrl(value) => {
                write!(
                    formatter,
                    "file URL cannot be converted to a local path: {value}"
                )
            }
            Self::InvalidFilePath(value) => write!(formatter, "invalid file path: {value}"),
        }
    }
}

impl std::error::Error for artifactSourceErr {}
