use burncloud_download_aria2::{quick_start, DownloadOptions};

use crate::error::{DownloadError, Result};
use crate::manager::DownloadManager;
use crate::utils::extract_filename_from_url;

impl DownloadManager {
    pub async fn new() -> Result<Self> {
        let aria2 = std::sync::Arc::new(quick_start().await?);
        let db = std::sync::Arc::new(burncloud_database_sys::DownloadDB::new().await?);
        let manager = Self { aria2, db };
        manager.restore_incomplete_downloads().await?;
        Ok(manager)
    }

    pub async fn add_download(&self, url: &str, download_dir: Option<&str>) -> Result<String> {
        let client = self.aria2.create_rpc_client().ok_or_else(|| {
            DownloadError::Aria2(burncloud_download_aria2::Aria2Error::RpcError(
                "客户端未就绪".to_string(),
            ))
        })?;

        let dir = download_dir.unwrap_or("./downloads").to_string();

        // 从 URL 中提取文件名
        let filename = extract_filename_from_url(url);

        let options = DownloadOptions {
            dir: Some(dir.clone()),
            out: filename,
            split: None,
            max_connection_per_server: None,
            continue_download: Some(true),
            allow_overwrite: Some(true),     // 开启覆盖式下载
            auto_file_renaming: Some(false), // 关闭文件自动重命名
        };

        let gid = client.add_uri(vec![url.to_string()], Some(options)).await?;
        self.db
            .add(&gid, vec![url.to_string()], Some(&dir), Some(&filename))
            .await?;

        // 启动进度监控
        self.start_progress_monitor(&gid).await;

        Ok(gid)
    }

    pub async fn get_status(&self, gid: &str) -> Result<burncloud_download_aria2::DownloadStatus> {
        let client = self.aria2.create_rpc_client().ok_or_else(|| {
            DownloadError::Aria2(burncloud_download_aria2::Aria2Error::RpcError(
                "客户端未就绪".to_string(),
            ))
        })?;

        let status = client.tell_status(gid).await?;

        // 同步状态到数据库
        self.db.update_status(gid, &status.status).await?;
        let total: i64 = status.total_length.parse().unwrap_or(0);
        let completed: i64 = status.completed_length.parse().unwrap_or(0);
        let speed: i64 = status.download_speed.parse().unwrap_or(0);
        self.db
            .update_progress(gid, total, completed, speed)
            .await?;

        Ok(status)
    }

    pub async fn pause(&self, gid: &str) -> Result<()> {
        let client = self.aria2.create_rpc_client().ok_or_else(|| {
            DownloadError::Aria2(burncloud_download_aria2::Aria2Error::RpcError(
                "客户端未就绪".to_string(),
            ))
        })?;

        client.pause(gid).await?;
        self.db.update_status(gid, "paused").await?;
        Ok(())
    }

    pub async fn resume(&self, gid: &str) -> Result<()> {
        let client = self.aria2.create_rpc_client().ok_or_else(|| {
            DownloadError::Aria2(burncloud_download_aria2::Aria2Error::RpcError(
                "客户端未就绪".to_string(),
            ))
        })?;

        client.unpause(gid).await?;
        self.db.update_status(gid, "active").await?;
        Ok(())
    }

    pub async fn remove(&self, gid: &str) -> Result<()> {
        let client = self.aria2.create_rpc_client().ok_or_else(|| {
            DownloadError::Aria2(burncloud_download_aria2::Aria2Error::RpcError(
                "客户端未就绪".to_string(),
            ))
        })?;

        client.remove(gid).await?;
        self.db.delete(gid).await?;
        Ok(())
    }
}
