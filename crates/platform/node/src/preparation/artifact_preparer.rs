use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use burncloud_download::DownloadManager;
use md5::Context as Md5Context;
use sha1::Sha1;
use sha2::{Digest as HashDigest, Sha256, Sha384, Sha512};
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;
use url::Url;

use super::artifact_request::ArtifactRequest;
use super::{
    ArtifactPrepareError, ArtifactVerificationStatus, Digest, DigestAlgorithm, PreparedArtifact,
};

/// 由 Node 组合根使用的工件准备端口。
#[async_trait]
pub trait ArtifactPreparation: Send + Sync {
    /// 准备请求中的工件，并返回本地工件或准备错误。
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use burncloud_node_runtime::{ArtifactPreparation, Digest};
    /// use url::Url;
    ///
    /// # async fn example<P: ArtifactPreparation>(
    /// #     preparer: &P,
    /// # ) -> Result<(), Box<dyn std::error::Error>> {
    /// let _artifact = preparer
    ///     .prepare(
    ///         Url::parse("https://modelscope.cn/models/example/resolve/main/model.gguf")?,
    ///         Digest::parse(
    ///             "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    ///         )?,
    ///     )
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # 注意
    ///
    /// 实现必须拒绝不支持的来源，并且不能把未通过摘要校验的路径作为成功回执返回。
    async fn prepare(
        &self,
        url: Url,
        expected_digest: Digest,
    ) -> Result<PreparedArtifact, ArtifactPrepareError>;
}

/// Node 侧管理下载器生命周期的真实工件准备器。
pub struct ArtifactPreparer {
    lifecycle: Mutex<ArtifactPreparerLifecycle>,
}

#[derive(Default)]
struct ArtifactPreparerLifecycle {
    download_manager: Option<Arc<DownloadManager>>,
    ref_count: usize,
}

impl ArtifactPreparer {
    /// 创建尚未启动下载管理器的工件准备器。
    ///
    /// # 示例
    ///
    /// ```
    /// use burncloud_node_runtime::ArtifactPreparer;
    ///
    /// let _preparer = ArtifactPreparer::new();
    /// ```
    ///
    /// # 注意
    ///
    /// 下载管理器只会在首个准备请求进入时启动，并在最后一个请求释放后停止。
    pub fn new() -> Self {
        Self {
            lifecycle: Mutex::new(ArtifactPreparerLifecycle::default()),
        }
    }

    /// 下载并校验指定的远程工件。
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use burncloud_node_runtime::{ArtifactPreparer, Digest};
    /// use url::Url;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let preparer = ArtifactPreparer::new();
    /// let artifact = preparer
    ///     .prepare(
    ///         Url::parse("https://modelscope.cn/models/example/resolve/main/model.gguf")?,
    ///         Digest::parse(
    ///             "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    ///         )?,
    ///     )
    ///     .await?;
    /// assert!(artifact.is_verified());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # 注意
    ///
    /// 只接受受支持主机名的 HTTP(S) URL。摘要不匹配时最多重新下载并校验一次；
    /// 两次校验均失败会返回 `Failed` 回执而不是未校验路径。
    pub async fn prepare(
        &self,
        url: Url,
        expected_digest: Digest,
    ) -> Result<PreparedArtifact, ArtifactPrepareError> {
        let source = super::ArtifactSource::try_from(&url)
            .map_err(|error| ArtifactPrepareError::PrepareFailed(error.to_string()))?;
        let request = ArtifactRequest::new(source, expected_digest);
        let manager = self.acquire_manager().await?;
        let result = self.prepare_with_manager(&manager, &request).await;
        // 先释放当前请求持有的引用，再让生命周期计数归零，确保管理器可以立即停止。
        drop(manager);
        self.release_manager().await;
        result
    }

    async fn acquire_manager(&self) -> Result<Arc<DownloadManager>, ArtifactPrepareError> {
        let mut lifecycle = self.lifecycle.lock().await;
        lifecycle.ref_count += 1;
        if lifecycle.ref_count == 1 {
            match DownloadManager::new().await {
                Ok(manager) => {
                    lifecycle.download_manager = Some(Arc::new(manager));
                }
                Err(error) => {
                    lifecycle.ref_count = 0;
                    return Err(ArtifactPrepareError::PrepareFailed(error.to_string()));
                }
            }
        }

        lifecycle.download_manager.as_ref().cloned().ok_or_else(|| {
            // 只有管理器启动失败才可能破坏该状态；这里保留明确错误，避免静默返回空引用。
            ArtifactPrepareError::PrepareFailed(
                "download manager lifecycle state is incomplete".to_string(),
            )
        })
    }

    async fn release_manager(&self) {
        let mut lifecycle = self.lifecycle.lock().await;
        assert!(
            lifecycle.ref_count > 0,
            "artifact preparer reference count must be positive before release"
        );
        lifecycle.ref_count -= 1;
        if lifecycle.ref_count == 0 {
            lifecycle.download_manager = None;
        }
    }

    fn build_receipt(
        &self,
        path: PathBuf,
        verified: bool,
    ) -> Result<PreparedArtifact, ArtifactPrepareError> {
        let status = if verified {
            ArtifactVerificationStatus::Verified
        } else {
            ArtifactVerificationStatus::Failed
        };
        // 失败回执不能泄露已删除或待删除的下载路径，避免下游误用未验证文件。
        let receipt_path = if verified { path } else { PathBuf::new() };
        PreparedArtifact::new(receipt_path, status)
            .map_err(|error| ArtifactPrepareError::PrepareFailed(error.to_string()))
    }

    async fn download_artifact(
        &self,
        manager: &DownloadManager,
        request: &ArtifactRequest,
    ) -> Result<PathBuf, ArtifactPrepareError> {
        let path = match request.source() {
            super::ArtifactSource::Http(url) => manager
                .download(url.as_str(), "models")
                .await
                .map_err(|error| ArtifactPrepareError::PrepareFailed(error.to_string()))?,
        };
        if path.as_os_str().is_empty() || !path.is_absolute() {
            return Err(ArtifactPrepareError::PrepareFailed(
                "download manager returned an unavailable artifact path".to_string(),
            ));
        }
        Ok(path)
    }

    async fn verify_file_digest(
        &self,
        path: &Path,
        expected_digest: &Digest,
    ) -> Result<bool, ArtifactPrepareError> {
        let mut file = tokio::fs::File::open(path)
            .await
            .map_err(|error| ArtifactPrepareError::PrepareFailed(error.to_string()))?;
        let mut digest = FileDigest::new(expected_digest.algorithm());
        let mut buffer = vec![0_u8; 64 * 1024];

        loop {
            let bytes_read = file
                .read(&mut buffer)
                .await
                .map_err(|error| ArtifactPrepareError::PrepareFailed(error.to_string()))?;
            if bytes_read == 0 {
                break;
            }
            digest.update(&buffer[..bytes_read]);
        }

        Ok(digest.finalize() == expected_digest.value())
    }

    async fn prepare_with_manager(
        &self,
        manager: &DownloadManager,
        request: &ArtifactRequest,
    ) -> Result<PreparedArtifact, ArtifactPrepareError> {
        let first_path = self.download_artifact(manager, request).await?;
        if self
            .verify_file_digest(&first_path, request.expected_digest())
            .await?
        {
            return self.build_receipt(first_path, true);
        }

        tokio::fs::remove_file(&first_path)
            .await
            .map_err(|error| ArtifactPrepareError::PrepareFailed(error.to_string()))?;

        let retry_path = self.download_artifact(manager, request).await?;
        if self
            .verify_file_digest(&retry_path, request.expected_digest())
            .await?
        {
            return self.build_receipt(retry_path, true);
        }

        tokio::fs::remove_file(&retry_path)
            .await
            .map_err(|error| ArtifactPrepareError::PrepareFailed(error.to_string()))?;
        self.build_receipt(PathBuf::new(), false)
    }
}

#[async_trait]
impl ArtifactPreparation for ArtifactPreparer {
    async fn prepare(
        &self,
        url: Url,
        expected_digest: Digest,
    ) -> Result<PreparedArtifact, ArtifactPrepareError> {
        ArtifactPreparer::prepare(self, url, expected_digest).await
    }
}

enum FileDigest {
    Md5(Md5Context),
    Sha1(Sha1),
    Sha256(Sha256),
    Sha384(Sha384),
    Sha512(Sha512),
}

impl FileDigest {
    fn new(algorithm: DigestAlgorithm) -> Self {
        match algorithm {
            DigestAlgorithm::Md5 => Self::Md5(Md5Context::new()),
            DigestAlgorithm::Sha1 => Self::Sha1(Sha1::new()),
            DigestAlgorithm::Sha256 => Self::Sha256(Sha256::new()),
            DigestAlgorithm::Sha384 => Self::Sha384(Sha384::new()),
            DigestAlgorithm::Sha512 => Self::Sha512(Sha512::new()),
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Md5(context) => context.consume(bytes),
            Self::Sha1(digest) => digest.update(bytes),
            Self::Sha256(digest) => digest.update(bytes),
            Self::Sha384(digest) => digest.update(bytes),
            Self::Sha512(digest) => digest.update(bytes),
        }
    }

    fn finalize(self) -> String {
        match self {
            Self::Md5(context) => hex::encode(context.compute().0),
            Self::Sha1(digest) => hex::encode(digest.finalize()),
            Self::Sha256(digest) => hex::encode(digest.finalize()),
            Self::Sha384(digest) => hex::encode(digest.finalize()),
            Self::Sha512(digest) => hex::encode(digest.finalize()),
        }
    }
}

impl Default for ArtifactPreparer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "../../tests/support/artifact_preparer_lifecycle.rs"]
mod lifecycle_tests;

#[cfg(test)]
#[path = "../../tests/support/artifact_preparer_receipt.rs"]
mod receipt_tests;

#[cfg(test)]
#[path = "../../tests/support/artifact_preparer_digest.rs"]
mod digest_tests;

#[cfg(test)]
#[path = "../../tests/support/artifact_preparer_prepare.rs"]
mod prepare_tests;

#[cfg(test)]
#[path = "../../tests/support/artifact_preparer_test_database.rs"]
mod test_database;
