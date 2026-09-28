use super::{ArtifactSource, Digest};

/// 下载请求信息，描述工件来源以及可选的完整性校验要求。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactRequest {
    pub source: ArtifactSource,
    pub expected_digest: Option<Digest>,
}
