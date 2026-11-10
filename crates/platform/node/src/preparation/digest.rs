use std::str::FromStr;

use super::{DigestAlgorithm, DigestError};

/// 规范化为小写十六进制的文件摘要。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Digest {
    algorithm: DigestAlgorithm,
    value: String,
}

impl Digest {
    /// 返回摘要使用的算法。
    ///
    /// # 示例
    ///
    /// ```
    /// use burncloud_node_runtime::{Digest, DigestAlgorithm};
    ///
    /// let digest = Digest::parse(
    ///     "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    /// )?;
    /// assert_eq!(digest.algorithm(), DigestAlgorithm::Sha256);
    /// # Ok::<(), burncloud_node_runtime::DigestError>(())
    /// ```
    pub const fn algorithm(&self) -> DigestAlgorithm {
        // 算法类型实现 Copy，因此返回值不会暴露内部存储的可变引用。
        self.algorithm
    }

    /// 返回规范化后的十六进制摘要值。
    ///
    /// # 示例
    ///
    /// ```
    /// use burncloud_node_runtime::Digest;
    ///
    /// let digest = Digest::parse(&format!("sha256:{}", "A".repeat(64)))?;
    /// assert_eq!(digest.value(), "a".repeat(64));
    /// # Ok::<(), burncloud_node_runtime::DigestError>(())
    /// ```
    pub fn value(&self) -> &str {
        // 仅返回共享字符串切片，调用方不能直接替换内部摘要值。
        &self.value
    }

    /// 创建并校验一个摘要值。
    ///
    /// 摘要值必须具有算法对应的十六进制字符数，并且只能包含 ASCII
    /// 十六进制字符；通过校验后会统一转换为小写。
    ///
    /// # 示例
    ///
    /// ```
    /// use burncloud_node_runtime::{Digest, DigestAlgorithm};
    ///
    /// let digest = Digest::new(DigestAlgorithm::Md5, "0123456789ABCDEF0123456789ABCDEF")?;
    /// assert_eq!(digest.value(), "0123456789abcdef0123456789abcdef");
    /// # Ok::<(), burncloud_node_runtime::DigestError>(())
    /// ```
    pub fn new(algorithm: DigestAlgorithm, value: impl AsRef<str>) -> Result<Self, DigestError> {
        let value = value.as_ref();
        if value.len() != algorithm.expected_hex_length() {
            return Err(DigestError::BadLength {
                expected: algorithm.expected_hex_length(),
                actual: value.len(),
            });
        }
        // 先检查长度，再检查字符集，以便错误信息明确反映输入问题。
        if !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(DigestError::NotHex);
        }
        Ok(Self {
            algorithm,
            value: value.to_ascii_lowercase(),
        })
    }

    /// 从 `algorithm:digest` 格式解析摘要。
    ///
    /// # 示例
    ///
    /// ```
    /// use burncloud_node_runtime::Digest;
    ///
    /// let digest = Digest::parse(
    ///     "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    /// )?;
    /// assert_eq!(digest.to_string(), "sha256:0000000000000000000000000000000000000000000000000000000000000000");
    /// # Ok::<(), burncloud_node_runtime::DigestError>(())
    /// ```
    pub fn parse(value: &str) -> Result<Self, DigestError> {
        // 先分离算法和摘要值，再分别执行算法解析与摘要内容校验。
        let (algorithm, digest) = value.split_once(':').ok_or(DigestError::MissingAlgorithm)?;
        Self::new(DigestAlgorithm::from_str(algorithm)?, digest)
    }
}

impl FromStr for Digest {
    type Err = DigestError;

    /// 从 `algorithm:digest` 格式解析摘要。
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // 复用 parse 的校验逻辑，保证两种入口具有完全一致的行为。
        Self::parse(value)
    }
}

impl std::fmt::Display for Digest {
    /// 使用规范化的 `algorithm:digest` 格式格式化摘要。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 始终输出规范化算法名和小写摘要，保持序列化结果稳定。
        write!(f, "{}:{}", self.algorithm.name(), self.value)
    }
}
