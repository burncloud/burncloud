use super::{ArtifactSource, Digest};

/// 描述工件来源以及可选完整性校验要求的下载请求。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactRequest {
    source: ArtifactSource,
    expected_digest: Option<Digest>,
}

impl ArtifactRequest {
    /// 创建一个工件请求。
    ///
    /// `source` 指定工件的位置，`expected_digest` 在存在时用于校验下载内容的完整性。
    pub fn new(source: ArtifactSource, expected_digest: Option<Digest>) -> Self {
        // 所有请求状态在构造时一次性写入，避免创建不完整的请求值。
        Self {
            source,
            expected_digest,
        }
    }

    /// 返回工件来源的只读引用。
    pub const fn source(&self) -> &ArtifactSource {
        // 仅暴露共享引用，调用方不能替换请求中的来源。
        &self.source
    }

    /// 返回预期摘要的只读引用；未配置摘要时返回 `None`。
    pub const fn expected_digest(&self) -> Option<&Digest> {
        // 将拥有的摘要借用为只读引用，保持请求的所有权不变。
        self.expected_digest.as_ref()
    }
}

impl std::fmt::Display for ArtifactRequest {
    /// 以来源和可选预期摘要组成的可读格式输出请求。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 无摘要时只输出来源，有摘要时追加校验信息。
        match self.expected_digest() {
            Some(digest) => write!(formatter, "{} (expected digest: {digest})", self.source),
            None => self.source.fmt(formatter),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA256: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    fn constructs_requests_and_exposes_read_only_accessors() {
        let source = ArtifactSource::parse("models/model.gguf").unwrap();
        let digest = Digest::parse(SHA256).unwrap();
        let request = ArtifactRequest::new(source.clone(), Some(digest.clone()));

        assert_eq!(request.source(), &source);
        assert_eq!(request.expected_digest(), Some(&digest));
    }

    #[test]
    fn displays_source_with_optional_digest() {
        let source = ArtifactSource::parse("models/model.gguf").unwrap();
        assert_eq!(
            ArtifactRequest::new(source.clone(), None).to_string(),
            "models/model.gguf"
        );
        assert_eq!(
            ArtifactRequest::new(source, Some(Digest::parse(SHA256).unwrap())).to_string(),
            format!("models/model.gguf (expected digest: {SHA256})")
        );
    }
}
