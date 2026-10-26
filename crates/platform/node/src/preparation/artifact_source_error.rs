use std::fmt;

/// `ArtifactSource` 解析失败时返回的错误类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactSourceError {
    /// 输入为空或只包含空白字符。
    Empty,
    /// URL 文本无法被解析。
    InvalidUrl(String),
    /// URL 使用了不支持的协议。
    UnsupportedScheme(String),
    /// URL 主机名不在受支持的精确列表中。
    UnsupportedHost(String),
}

impl fmt::Display for ArtifactSourceError {
    /// 将来源解析错误转换为可读文本。
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("artifact source cannot be empty"),
            Self::InvalidUrl(value) => write!(formatter, "invalid URL: {value}"),
            Self::UnsupportedScheme(scheme) => {
                write!(formatter, "unsupported artifact source scheme: {scheme}")
            }
            Self::UnsupportedHost(host) => {
                write!(formatter, "unsupported artifact source host: {host}")
            }
        }
    }
}

impl std::error::Error for ArtifactSourceError {}
