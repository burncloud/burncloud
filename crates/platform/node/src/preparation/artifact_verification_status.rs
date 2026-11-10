/// 描述工件完整性校验是否成功的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactVerificationStatus {
    /// 工件已完成校验且校验成功。
    Verified,
    /// 工件已完成校验但校验失败。
    Failed,
}
