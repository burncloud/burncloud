use std::str::FromStr;

use url::Url;

use super::artifact_source_error::ArtifactSourceError;

/// 已通过协议和主机名校验的远程工件来源。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArtifactSource {
    /// 使用受支持主机名的 HTTP 或 HTTPS 地址。
    Http(Url),
}

impl ArtifactSource {
    /// 将 URL 文本解析为受支持的远程工件来源。
    ///
    /// # 示例
    ///
    /// ```
    /// use burncloud_node_runtime::ArtifactSource;
    ///
    /// let source = ArtifactSource::parse(
    ///     "https://huggingface.com/models/example/resolve/main/model.gguf",
    /// )?;
    /// assert!(source.is_http());
    /// # Ok::<(), burncloud_node_runtime::ArtifactSourceError>(())
    /// ```
    ///
    /// # 注意
    ///
    /// 只接受 `http` 和 `https`，且主机名必须精确匹配受支持的来源列表。
    pub fn parse(value: impl AsRef<str>) -> Result<Self, ArtifactSourceError> {
        let value = value.as_ref().trim();
        if value.is_empty() {
            return Err(ArtifactSourceError::Empty);
        }
        Self::parse_url(value)
    }

    /// 返回远程来源中的 URL。
    pub fn as_http(&self) -> &Url {
        match self {
            Self::Http(url) => url,
        }
    }

    /// 判断来源是否为 HTTP(S) URL。
    pub const fn is_http(&self) -> bool {
        matches!(self, Self::Http(_))
    }

    fn parse_url(value: &str) -> Result<Self, ArtifactSourceError> {
        let url = Url::parse(value)
            .map_err(|error| ArtifactSourceError::InvalidUrl(error.to_string()))?;
        match url.scheme().to_ascii_lowercase().as_str() {
            "http" | "https" => {
                let host = url.host_str().unwrap_or_default();
                Self::validate_allowed_host(host)?;
                Ok(Self::Http(url))
            }
            scheme => Err(ArtifactSourceError::UnsupportedScheme(scheme.to_owned())),
        }
    }

    fn validate_allowed_host(host: &str) -> Result<(), ArtifactSourceError> {
        match host {
            "modelscope.cn" | "hf-mirror.com" | "huggingface.com" | "huggingface.co" => Ok(()),
            _ => Err(ArtifactSourceError::UnsupportedHost(host.to_owned())),
        }
    }
}

impl FromStr for ArtifactSource {
    type Err = ArtifactSourceError;

    /// 从 URL 文本解析远程工件来源。
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl std::fmt::Display for ArtifactSource {
    /// 使用规范化 URL 格式输出来源。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_http().fmt(formatter)
    }
}

impl TryFrom<&Url> for ArtifactSource {
    type Error = ArtifactSourceError;

    /// 将 URL 转换为受支持的远程来源。
    fn try_from(value: &Url) -> Result<Self, Self::Error> {
        Self::parse(value.as_str())
    }
}
