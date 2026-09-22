use std::sync::Arc;
use tokio::time::{sleep, Duration};

use crate::error::{DownloadError, Result};
use crate::manager::DownloadManager;

impl DownloadManager {
    pub async fn start_progress_monitor(&self, gid: &str) {
        let aria2 = Arc::clone(&self.aria2);
        let db = Arc::clone(&self.db);
        let gid = gid.to_string();

        tokio::spawn(async move {
            loop {
                if let Some(client) = aria2.create_rpc_client() {
                    if let Ok(status) = client.tell_status(&gid).await {
                        let _ = db.update_status(&gid, &status.status).await;
                        let total: i64 = status.total_length.parse().unwrap_or(0);
                        let completed: i64 = status.completed_length.parse().unwrap_or(0);
                        let speed: i64 = status.download_speed.parse().unwrap_or(0);
                        let _ = db.update_progress(&gid, total, completed, speed).await;

                        // 如果下载完成或出错，停止监控
                        if status.status == "complete" || status.status == "error" {
                            break;
                        }
                    }
                } else {
                    break; // 客户端不可用，停止监控
                }
                sleep(Duration::from_secs(2)).await;
            }
        });
    }

    pub async fn restore_incomplete_downloads(&self) -> Result<Vec<String>> {
        let client = self.aria2.create_rpc_client().ok_or_else(|| {
            DownloadError::Aria2(burncloud_download_aria2::Aria2Error::RpcError(
                "客户端未就绪".to_string(),
            ))
        })?;

        let incomplete = self.db.list(Some("active")).await?;
        let mut restored = Vec::new();

        for download in incomplete {
            let uris: Vec<String> = serde_json::from_str(&download.uris).unwrap_or_default();
            if !uris.is_empty() {
                let dir = download
                    .download_dir
                    .as_deref()
                    .unwrap_or("./downloads")
                    .to_string();
                let options = burncloud_download_aria2::DownloadOptions {
                    dir: Some(dir),
                    out: download.filename,
                    split: None,
                    max_connection_per_server: None,
                    continue_download: Some(true),
                    allow_overwrite: Some(true),     // 开启覆盖式下载
                    auto_file_renaming: Some(false), // 关闭文件自动重命名
                };
                if let Ok(new_gid) = client.add_uri(uris, Some(options)).await {
                    // 更新数据库中的gid为新的gid
                    let _ = self.db.update_gid(&download.gid, &new_gid).await;
                    // 启动进度监控
                    self.start_progress_monitor(&new_gid).await;
                    restored.push(new_gid);
                }
            }
        }

        Ok(restored)
    }
}
