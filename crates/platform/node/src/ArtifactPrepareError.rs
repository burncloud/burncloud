/// 工件准备过程返回的错误。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ArtifactPrepareError {
    #[error("artifact preparation failed: {0}")]
    PrepareFailed(String),
}
