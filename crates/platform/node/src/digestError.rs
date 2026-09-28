/// 摘要校验失败的具体原因。
#[derive(Debug, PartialEq, Eq)]
pub enum DigestError {
    MissingAlgorithm,
    UnsupportedAlgorithm(String),
    NotHex,
    BadLength { expected: usize, actual: usize },
}

impl std::fmt::Display for DigestError {
    /// 将摘要校验错误转换为可读的错误信息。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingAlgorithm => f.write_str("digest algorithm is missing"),
            Self::UnsupportedAlgorithm(value) => write!(f, "unsupported digest algorithm: {value}"),
            Self::NotHex => f.write_str("digest value is not hexadecimal"),
            Self::BadLength { expected, actual } => {
                write!(
                    f,
                    "digest length must be {expected} hexadecimal characters, got {actual}"
                )
            }
        }
    }
}

impl std::error::Error for DigestError {}
