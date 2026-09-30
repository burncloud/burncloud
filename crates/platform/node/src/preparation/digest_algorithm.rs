use std::str::FromStr;

use super::DigestError;

/// 当前工件摘要校验接口支持的算法。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DigestAlgorithm {
    /// MD5，输出 16 字节摘要。
    Md5,
    /// SHA-1，输出 20 字节摘要。
    Sha1,
    /// SHA-256，输出 32 字节摘要。
    Sha256,
    /// SHA-384，输出 48 字节摘要。
    Sha384,
    /// SHA-512，输出 64 字节摘要。
    Sha512,
}

impl DigestAlgorithm {
    /// 返回算法的规范化名称。
    pub const fn name(self) -> &'static str {
        // 返回固定字符串，避免调用方依赖枚举变体的调试格式。
        match self {
            Self::Md5 => "md5",
            Self::Sha1 => "sha1",
            Self::Sha256 => "sha256",
            Self::Sha384 => "sha384",
            Self::Sha512 => "sha512",
        }
    }

    /// 返回算法输出摘要的字节数。
    pub const fn digest_bytes(self) -> usize {
        // 结果表示算法原始摘要的字节数，不是十六进制文本长度。
        match self {
            Self::Md5 => 16,
            Self::Sha1 => 20,
            Self::Sha256 => 32,
            Self::Sha384 => 48,
            Self::Sha512 => 64,
        }
    }

    /// 返回摘要编码为十六进制字符串后的字符数。
    pub const fn expected_hex_length(self) -> usize {
        // 每个字节对应两个十六进制字符，因此长度是摘要字节数的两倍。
        match self {
            Self::Md5 => 32,
            Self::Sha1 => 40,
            Self::Sha256 => 64,
            Self::Sha384 => 96,
            Self::Sha512 => 128,
        }
    }
}

impl FromStr for DigestAlgorithm {
    type Err = DigestError;

    /// 从算法名称解析摘要算法。
    ///
    /// 解析会忽略大小写和连接符，因此 `sha256`、`SHA256`、`sha-256`
    /// 与 `SHA-256` 会解析为同一个算法；SHA-1 与 SHA-384 同样适用。
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // 先统一大小写，再移除连接符，以支持常见的算法名称写法。
        let normalized = value.to_ascii_lowercase().replace('-', "");
        match normalized.as_str() {
            "md5" => Ok(Self::Md5),
            "sha1" => Ok(Self::Sha1),
            "sha256" => Ok(Self::Sha256),
            "sha384" => Ok(Self::Sha384),
            "sha512" => Ok(Self::Sha512),
            _ => Err(DigestError::UnsupportedAlgorithm(normalized)),
        }
    }
}

impl std::fmt::Display for DigestAlgorithm {
    /// 使用算法的规范化名称进行格式化。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 使用规范化名称，保证显示结果与解析后的算法名称一致。
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_digest_and_hex_lengths() {
        assert_eq!(DigestAlgorithm::Md5.digest_bytes(), 16);
        assert_eq!(DigestAlgorithm::Md5.expected_hex_length(), 32);
        assert_eq!(DigestAlgorithm::Sha1.digest_bytes(), 20);
        assert_eq!(DigestAlgorithm::Sha1.expected_hex_length(), 40);
        assert_eq!(DigestAlgorithm::Sha256.digest_bytes(), 32);
        assert_eq!(DigestAlgorithm::Sha256.expected_hex_length(), 64);
        assert_eq!(DigestAlgorithm::Sha384.digest_bytes(), 48);
        assert_eq!(DigestAlgorithm::Sha384.expected_hex_length(), 96);
        assert_eq!(DigestAlgorithm::Sha512.digest_bytes(), 64);
        assert_eq!(DigestAlgorithm::Sha512.expected_hex_length(), 128);
    }

    #[test]
    fn parses_common_algorithm_aliases() {
        for alias in ["sha256", "SHA256", "sha-256", "SHA-256"] {
            assert_eq!(alias.parse(), Ok(DigestAlgorithm::Sha256));
        }
        for alias in ["sha1", "SHA1", "sha-1", "SHA-1"] {
            assert_eq!(alias.parse(), Ok(DigestAlgorithm::Sha1));
        }
        for alias in ["sha384", "SHA384", "sha-384", "SHA-384"] {
            assert_eq!(alias.parse(), Ok(DigestAlgorithm::Sha384));
        }
    }

    #[test]
    fn displays_canonical_algorithm_names() {
        assert_eq!(DigestAlgorithm::Md5.to_string(), "md5");
        assert_eq!(DigestAlgorithm::Sha1.to_string(), "sha1");
        assert_eq!(DigestAlgorithm::Sha256.to_string(), "sha256");
        assert_eq!(DigestAlgorithm::Sha384.to_string(), "sha384");
        assert_eq!(DigestAlgorithm::Sha512.to_string(), "sha512");
    }
}
