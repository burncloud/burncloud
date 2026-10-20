use std::sync::Arc;

use async_trait::async_trait;
use burncloud_download::DownloadManager;
use tokio::sync::Mutex;
use url::Url;

use super::{ArtifactPrepareError, Digest, PreparedArtifact};

/// 由 Node 组合根使用的工件准备端口。
#[async_trait]
pub trait ArtifactPreparation: Send + Sync {
    /// 准备请求中的工件，并返回本地工件或准备错误。
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
}

impl Default for ArtifactPreparer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "../../tests/support/artifact_preparer_lifecycle.rs"]
mod lifecycle_tests;
