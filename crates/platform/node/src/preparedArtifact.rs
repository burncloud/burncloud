use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// 一个本地可用的构件及其完整性校验状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedArtifact {
    local_path: PathBuf,
    verified: bool,
}

impl PreparedArtifact {
    /// 创建一个已准备好的构件，并将其路径以当前平台的绝对文件管理器格式存储起来。
    pub fn new(local_path: impl AsRef<Path>, verified: bool) -> io::Result<Self> {
        // 相对路径会相对进程的工作目录进行解析，从而在该值离开此构造函数前，
        // 每个存储的路径都是绝对路径。
        let path = local_path.as_ref();
        let absolute_path = match path.is_absolute() {
            true => path.to_path_buf(),
            false => std::env::current_dir()?.join(path),
        };

        Ok(Self {
            local_path: absolute_path,
            verified,
        })
    }

    /// 返回已准备构件的绝对本地路径。
    pub fn local_path(&self) -> &Path {
        // 返回一个共享引用，使调用方无法替换存储的路径。
        &self.local_path
    }

    /// 返回构件是否通过了完整性校验。
    pub const fn verified(&self) -> bool {
        // 仅暴露校验位，而不暴露可变字段访问。
        self.verified
    }

    /// 返回构件是否通过了完整性校验。
    pub const fn is_verified(&self) -> bool {
        // 为偏好布尔命名风格的调用方保留谓词式别名。
        self.verified
    }
}

impl fmt::Display for PreparedArtifact {
    /// 格式化绝对本地路径和校验状态，用于诊断输出。
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 使用 Path::display，使分隔符和盘符前缀遵循宿主操作系统。
        write!(
            formatter,
            "{} (verified: {})",
            self.local_path.display(),
            self.verified
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_resolves_relative_paths_to_absolute_paths() {
        let artifact = PreparedArtifact::new("models/model.gguf", true).unwrap();

        assert!(artifact.local_path().is_absolute());
        assert!(artifact.verified());
        assert!(artifact.is_verified());
    }

    #[test]
    fn display_includes_path_and_verification_state() {
        let path = std::env::temp_dir().join("model.gguf");
        let artifact = PreparedArtifact::new(&path, false).unwrap();

        assert_eq!(
            artifact.to_string(),
            format!("{} (verified: false)", path.display())
        );
    }
}
