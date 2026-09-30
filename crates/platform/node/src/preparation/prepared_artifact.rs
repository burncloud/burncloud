use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use super::artifact_verification_status::ArtifactVerificationStatus;

/// 一个本地可用的工件及其完整性校验状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedArtifact {
    local_path: PathBuf,
    verification_status: ArtifactVerificationStatus,
}

impl PreparedArtifact {
    /// 创建一个已准备好的构件，并存储路径与完整性校验状态。
    pub fn new(
        local_path: impl AsRef<Path>,
        verification_status: ArtifactVerificationStatus,
    ) -> io::Result<Self> {
        // 相对路径会相对进程的工作目录进行解析，从而在该值离开此构造函数前，
        // 每个存储的路径都是绝对路径。
        let path = local_path.as_ref();
        // 统一保存绝对路径，避免后续工作目录变化导致回执指向不同位置。
        let absolute_path = match path.is_absolute() {
            true => path.to_path_buf(),
            false => std::env::current_dir()?.join(path),
        };

        Ok(Self {
            local_path: absolute_path,
            verification_status,
        })
    }

    /// 返回已准备构件的绝对本地路径。
    pub fn local_path(&self) -> &Path {
        // 返回一个共享引用，使调用方无法替换存储的路径。
        &self.local_path
    }

    /// 返回工件完整性校验的三态结果。
    pub const fn verification_status(&self) -> ArtifactVerificationStatus {
        // 状态类型实现 Copy，因此可以按值返回而不转移工件所有权。
        self.verification_status
    }

    /// 返回校验状态的布尔映射；校验成功或未要求校验时返回 `true`，校验失败时返回 `false`。
    pub const fn is_verified(&self) -> bool {
        // 只有校验失败映射为 false，成功与未要求校验均映射为 true。
        matches!(
            self.verification_status,
            ArtifactVerificationStatus::Verified | ArtifactVerificationStatus::NotRequired
        )
    }
}

impl fmt::Display for PreparedArtifact {
    /// 格式化绝对本地路径和校验状态，用于诊断输出。
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 使用 Path::display，使分隔符和盘符前缀遵循宿主操作系统。
        let verification_status = match self.verification_status {
            ArtifactVerificationStatus::Verified => "verified",
            ArtifactVerificationStatus::Failed => "failed",
            ArtifactVerificationStatus::NotRequired => "not required",
        };
        write!(
            formatter,
            "{} (verification: {})",
            self.local_path.display(),
            verification_status
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_resolves_relative_paths_to_absolute_paths() {
        let artifact =
            PreparedArtifact::new("models/model.gguf", ArtifactVerificationStatus::Verified)
                .unwrap();

        assert!(artifact.local_path().is_absolute());
        assert!(artifact.is_verified());
        assert_eq!(
            artifact.verification_status(),
            ArtifactVerificationStatus::Verified
        );
    }

    #[test]
    fn display_includes_path_and_verification_state() {
        let path = std::env::temp_dir().join("model.gguf");
        let artifact = PreparedArtifact::new(&path, ArtifactVerificationStatus::Failed).unwrap();

        assert_eq!(
            artifact.to_string(),
            format!("{} (verification: failed)", path.display())
        );
    }

    #[test]
    fn verification_status_preserves_all_three_states() {
        let path = std::env::temp_dir().join("model.gguf");
        let statuses = [
            (ArtifactVerificationStatus::Verified, "verified", true),
            (ArtifactVerificationStatus::Failed, "failed", false),
            (
                ArtifactVerificationStatus::NotRequired,
                "not required",
                true,
            ),
        ];

        for (status, display_status, is_verified) in statuses {
            let artifact = PreparedArtifact::new(&path, status).unwrap();

            assert_eq!(artifact.verification_status(), status);
            assert_eq!(artifact.is_verified(), is_verified);
            assert_eq!(
                artifact.to_string(),
                format!("{} (verification: {display_status})", path.display())
            );
        }
    }
}
