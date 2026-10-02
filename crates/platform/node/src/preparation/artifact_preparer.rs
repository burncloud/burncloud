use async_trait::async_trait;

use super::{ArtifactPrepareError, ArtifactRequest, PreparedArtifact};

/// 定义将工件请求准备为可供本机使用的工件的异步接口。
#[async_trait]
pub trait ArtifactPreparer: Send + Sync {
    /// 准备请求中的工件，并返回本地工件或准备错误。
    async fn prepare(
        &self,
        request: ArtifactRequest,
    ) -> Result<PreparedArtifact, ArtifactPrepareError>;
}
