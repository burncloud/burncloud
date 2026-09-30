/// 工件准备过程失败时返回的错误类型。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ArtifactPrepareError {
    #[error("artifact preparation failed: {0}")]
    PrepareFailed(String),
}
