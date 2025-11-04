use burncloud_database_download::DownloadDB;
use burncloud_download_aria2::{quick_start, Aria2Manager};
use thiserror::Error;
use tokio::time::{sleep, Duration};
use std::sync::Arc;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("数据库错误: {0}")]
    Database(#[from] burncloud_database::DatabaseError),
    #[error("Aria2错误: {0}")]
    Aria2(#[from] burncloud_download_aria2::Aria2Error),
}

pub type Result<T> = std::result::Result<T, DownloadError>;

pub struct DownloadManager {
    aria2: Arc<Aria2Manager>,
    db: Arc<DownloadDB>,
}

impl DownloadManager {
    pub async fn new() -> Result<Self> {
        let aria2 = Arc::new(quick_start().await?);
        let db = Arc::new(DownloadDB::new().await?);
        let manager = Self { aria2, db };
        manager.restore_incomplete_downloads().await?;
        Ok(manager)
    }

    pub async fn add_download(&self, url: &str, download_dir: Option<&str>) -> Result<String> {
        let client = self.aria2.create_rpc_client().ok_or_else(||
            DownloadError::Aria2(burncloud_download_aria2::Aria2Error::RpcError("客户端未就绪".to_string())))?;

        let dir = download_dir.unwrap_or("./downloads").to_string();
        let options = burncloud_download_aria2::DownloadOptions {
            dir: Some(dir.clone()),
            out: None,
            split: None,
            max_connection_per_server: None,
            continue_download: Some(true),
        };

        let gid = client.add_uri(vec![url.to_string()], Some(options)).await?;
        self.db.add(&gid, vec![url.to_string()], Some(&dir), None).await?;

        // 启动进度监控
        self.start_progress_monitor(&gid).await;

        Ok(gid)
    }

    pub async fn get_status(&self, gid: &str) -> Result<burncloud_download_aria2::DownloadStatus> {
        let client = self.aria2.create_rpc_client().ok_or_else(||
            DownloadError::Aria2(burncloud_download_aria2::Aria2Error::RpcError("客户端未就绪".to_string())))?;

        let status = client.tell_status(gid).await?;

        // 同步状态到数据库
        self.db.update_status(gid, &status.status).await?;
        let total: i64 = status.total_length.parse().unwrap_or(0);
        let completed: i64 = status.completed_length.parse().unwrap_or(0);
        let speed: i64 = status.download_speed.parse().unwrap_or(0);
        self.db.update_progress(gid, total, completed, speed).await?;

        Ok(status)
    }

    pub async fn pause(&self, gid: &str) -> Result<()> {
        let client = self.aria2.create_rpc_client().ok_or_else(||
            DownloadError::Aria2(burncloud_download_aria2::Aria2Error::RpcError("客户端未就绪".to_string())))?;

        client.pause(gid).await?;
        self.db.update_status(gid, "paused").await?;
        Ok(())
    }

    pub async fn resume(&self, gid: &str) -> Result<()> {
        let client = self.aria2.create_rpc_client().ok_or_else(||
            DownloadError::Aria2(burncloud_download_aria2::Aria2Error::RpcError("客户端未就绪".to_string())))?;

        client.unpause(gid).await?;
        self.db.update_status(gid, "active").await?;
        Ok(())
    }

    pub async fn remove(&self, gid: &str) -> Result<()> {
        let client = self.aria2.create_rpc_client().ok_or_else(||
            DownloadError::Aria2(burncloud_download_aria2::Aria2Error::RpcError("客户端未就绪".to_string())))?;

        client.remove(gid).await?;
        self.db.delete(gid).await?;
        Ok(())
    }

    async fn start_progress_monitor(&self, gid: &str) {
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

    async fn restore_incomplete_downloads(&self) -> Result<Vec<String>> {
        let client = self.aria2.create_rpc_client().ok_or_else(||
            DownloadError::Aria2(burncloud_download_aria2::Aria2Error::RpcError("客户端未就绪".to_string())))?;

        let incomplete = self.db.list(Some("active")).await?;
        let mut restored = Vec::new();

        for download in incomplete {
            let uris: Vec<String> = serde_json::from_str(&download.uris).unwrap_or_default();
            if !uris.is_empty() {
                let dir = download.download_dir.as_deref().unwrap_or("./downloads").to_string();
                let options = burncloud_download_aria2::DownloadOptions {
                    dir: Some(dir),
                    out: download.filename,
                    split: None,
                    max_connection_per_server: None,
                    continue_download: Some(true),
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