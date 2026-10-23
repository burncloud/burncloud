use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use super::artifact_verification_status::ArtifactVerificationStatus;

/// 一个本地工件及其完整性校验状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedArtifact {
    local_path: PathBuf,
    verification_status: ArtifactVerificationStatus,
}

impl PreparedArtifact {
    /// 创建满足路径和校验状态不变量的工件回执。
    ///
    /// # 示例
    ///
    /// ```
    /// use burncloud_node_runtime::{
    ///     ArtifactVerificationStatus, PreparedArtifact,
    /// };
    ///
    /// let artifact = PreparedArtifact::new(
    ///     "models/model.gguf",
    ///     ArtifactVerificationStatus::Verified,
    /// )?;
    /// assert!(artifact.local_path().is_absolute());
    /// assert!(artifact.is_verified());
    /// # Ok::<(), std::io::Error>(())
    /// ```
    ///
    /// # 注意
    ///
    /// `Verified` 必须携带非空路径，`Failed` 必须携带空路径；成功回执中的相对路径
    /// 会基于当前工作目录转换为绝对路径。
    pub fn new(
        local_path: impl AsRef<Path>,
        verification_status: ArtifactVerificationStatus,
    ) -> io::Result<Self> {
        let path = local_path.as_ref();
        if verification_status == ArtifactVerificationStatus::Failed {
            if !path.as_os_str().is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "failed artifact must not contain a local path",
                ));
            }
            return Ok(Self {
                local_path: PathBuf::new(),
                verification_status,
            });
        }

        if path.as_os_str().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "verified artifact must contain a local path",
            ));
        }

        let absolute_path = match path.is_absolute() {
            true => path.to_path_buf(),
            false => std::env::current_dir()?.join(path),
        };
        Ok(Self {
            local_path: absolute_path,
            verification_status,
        })
    }

    /// 返回工件的绝对本地路径；失败回执返回空路径。
    ///
    /// # 示例
    ///
    /// ```
    /// use burncloud_node_runtime::{
    ///     ArtifactVerificationStatus, PreparedArtifact,
    /// };
    ///
    /// let artifact = PreparedArtifact::new("", ArtifactVerificationStatus::Failed)?;
    /// assert!(artifact.local_path().as_os_str().is_empty());
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn local_path(&self) -> &Path {
        &self.local_path
    }

    /// 返回工件的摘要校验状态。
    ///
    /// # 示例
    ///
    /// ```
    /// use burncloud_node_runtime::{
    ///     ArtifactVerificationStatus, PreparedArtifact,
    /// };
    ///
    /// let artifact = PreparedArtifact::new("model.gguf", ArtifactVerificationStatus::Verified)?;
    /// assert_eq!(artifact.verification_status(), ArtifactVerificationStatus::Verified);
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub const fn verification_status(&self) -> ArtifactVerificationStatus {
        self.verification_status
    }

    /// 判断工件是否通过摘要校验并允许后续流程使用。
    ///
    /// # 示例
    ///
    /// ```
    /// use burncloud_node_runtime::{
    ///     ArtifactVerificationStatus, PreparedArtifact,
    /// };
    ///
    /// let artifact = PreparedArtifact::new("model.gguf", ArtifactVerificationStatus::Verified)?;
    /// assert!(artifact.is_verified());
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub const fn is_verified(&self) -> bool {
        matches!(
            self.verification_status,
            ArtifactVerificationStatus::Verified
        )
    }
}

impl fmt::Display for PreparedArtifact {
    /// 格式化工件路径和校验状态，用于诊断输出。
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let verification_status = match self.verification_status {
            ArtifactVerificationStatus::Verified => "verified",
            ArtifactVerificationStatus::Failed => "failed",
        };
        write!(
            formatter,
            "{} (verification: {})",
            self.local_path.display(),
            verification_status
        )
    }
}
