//! Database persistence for download module

use burncloud_database::{Database, DatabaseError, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// 持久化下载任务。
#[derive(sqlx::FromRow, Serialize, Deserialize, Clone)]
pub struct SysDownload {
    /// 数据库稳定主键。
    pub id: i64,
    /// aria2 任务标识。
    pub gid: String,
    /// 单个下载地址；旧记录从 `uris` 恢复。
    pub url: Option<String>,
    /// 下载状态。
    pub status: String,
    /// URI JSON 数组，保留以兼容旧数据。
    pub uris: String,
    /// 文件总字节数。
    pub total_length: Option<i64>,
    /// 已完成字节数。
    pub completed_length: Option<i64>,
    /// 当前下载速度。
    pub download_speed: Option<i64>,
    /// 保存目录。
    pub download_dir: Option<String>,
    /// 输出文件名。
    pub filename: Option<String>,
    /// 连接数配置。
    pub connections: Option<i64>,
    /// 分段连接数配置。
    pub split: Option<i64>,
    /// 重试次数。
    pub retry_count: i64,
    /// 失败原因。
    pub error: Option<String>,
    /// 最终文件路径。
    pub final_path: Option<String>,
    /// 完成时间。
    pub completed_at: Option<String>,
    /// 创建时间。
    pub created_at: Option<String>,
    /// 更新时间。
    pub updated_at: Option<String>,
}

/// Download database
pub struct DownloadDB {
    db: Database,
}

impl DownloadDB {
    /// Create new download database
    pub async fn new() -> Result<Self> {
        let db = Database::new().await?;
        Ok(Self { db })
    }

    /// Wrap an initialized database connection.
    ///
    /// The counterpart of [`SettingDatabase::new_with_db`](crate::SettingDatabase::new_with_db), and it exists
    /// for the same reason: [`DownloadDB::new`] opens `Database::new()`, which honours `BURNCLOUD_DATABASE_URL`
    /// and otherwise uses the **shared default path** under the user's data directory. A test that used it would
    /// write download records into the developer's real database and could not run twice cleanly. This lets a
    /// caller -- a test, or a binary that already holds a connection -- supply the database instead.
    pub async fn new_with_db(db: Database) -> Result<Self> {
        Ok(Self { db })
    }

    fn now_string() -> String {
        Utc::now().to_rfc3339()
    }

    /// Add a new download record
    pub async fn add(
        &self,
        gid: &str,
        uris: Vec<String>,
        download_dir: Option<&str>,
        filename: Option<&str>,
    ) -> Result<()> {
        let uris_json = serde_json::to_string(&uris)?;
        let result = self
            .db
            .execute_query_with_params(
                r#"
                INSERT INTO sys_downloads (gid, status, uris, download_dir, filename, updated_at)
                VALUES (?, 'active', ?, ?, ?, ?)
                "#,
                vec![
                    gid.to_string(),
                    uris_json,
                    download_dir.unwrap_or("").to_string(),
                    filename.unwrap_or("").to_string(),
                    Self::now_string(),
                ],
            )
            .await?;
        ensure_row_updated(result, "新增下载记录")?;
        Ok(())
    }

    /// 新增下载任务并返回数据库稳定主键。
    pub async fn insert_task(
        &self,
        gid: &str,
        url: &str,
        download_dir: &str,
        filename: &str,
    ) -> Result<i64> {
        let uris = serde_json::to_string(&[url])?;
        let result = self
            .db
            .execute_query_with_params(
                r#"INSERT INTO sys_downloads
                   (gid, url, status, uris, download_dir, filename, updated_at)
                   VALUES (?, ?, 'pending', ?, ?, ?, ?)"#,
                vec![
                    gid.to_string(),
                    url.to_string(),
                    uris,
                    download_dir.to_string(),
                    filename.to_string(),
                    Self::now_string(),
                ],
            )
            .await?;
        ensure_row_updated(result, "新增下载记录")?;

        self.db
            .fetch_optional_with_params::<SysDownload>(
                "SELECT id, gid, url, status, uris, total_length, completed_length, download_speed, \
                 download_dir, filename, connections, split, retry_count, error, final_path, \
                 completed_at, created_at, updated_at FROM sys_downloads WHERE gid = ?",
                vec![gid.to_string()],
            )
            .await?
            .map(|record| record.id)
            .ok_or_else(|| burncloud_database::DatabaseError::Query(
                format!("download row was not found after inserting GID {gid}"),
            ))
    }

    /// 按 GID 读取任务。
    pub async fn get_by_gid(&self, gid: &str) -> Result<Option<SysDownload>> {
        self.db
            .fetch_optional_with_params::<SysDownload>(
                "SELECT id, gid, url, status, uris, total_length, completed_length, download_speed, \
                 download_dir, filename, connections, split, retry_count, error, final_path, \
                 completed_at, created_at, updated_at FROM sys_downloads WHERE gid = ?",
                vec![gid.to_string()],
            )
            .await
    }

    /// 列出需要恢复的任务。
    pub async fn list_incomplete(&self) -> Result<Vec<SysDownload>> {
        self.db
            .fetch_all::<SysDownload>(
                "SELECT id, gid, url, status, uris, total_length, completed_length, download_speed, \
                 download_dir, filename, connections, split, retry_count, error, final_path, \
                 completed_at, created_at, updated_at FROM sys_downloads \
                 WHERE status IN ('pending', 'active', 'waiting', 'paused') ORDER BY id",
            )
            .await
    }

    /// 查找相同 URL、目录和文件名的已完成任务。
    pub async fn find_completed(
        &self,
        url: &str,
        download_dir: &str,
        filename: &str,
    ) -> Result<Option<SysDownload>> {
        self.db
            .fetch_optional_with_params::<SysDownload>(
                "SELECT id, gid, url, status, uris, total_length, completed_length, download_speed, \
                 download_dir, filename, connections, split, retry_count, error, final_path, \
                 completed_at, created_at, updated_at FROM sys_downloads \
                 WHERE url = ? AND download_dir = ? AND filename = ? \
                   AND status = 'completed' AND final_path IS NOT NULL ORDER BY id DESC LIMIT 1",
                vec![
                    url.to_string(),
                    download_dir.to_string(),
                    filename.to_string(),
                ],
            )
            .await
    }

    /// 更新 GID，并保留稳定数据库主键。
    pub async fn update_gid_by_id(&self, id: i64, gid: &str, retry_count: i64) -> Result<()> {
        let result = self
            .db
            .execute_query_with_params(
                "UPDATE sys_downloads SET gid = ?, retry_count = ?, status = 'pending', \
                 error = NULL, updated_at = ? WHERE id = ?",
                vec![
                    gid.to_string(),
                    retry_count.to_string(),
                    Self::now_string(),
                    id.to_string(),
                ],
            )
            .await?;
        ensure_row_updated(result, "GID 和数据库 ID")?;
        Ok(())
    }

    /// 更新任务状态和进度。
    pub async fn update_task_progress(
        &self,
        id: i64,
        gid: &str,
        status: &str,
        progress: (i64, i64, i64),
    ) -> Result<()> {
        let (total_length, completed_length, download_speed) = progress;
        let result = self
            .db
            .execute_query_with_params(
                "UPDATE sys_downloads SET status = ?, total_length = ?, completed_length = ?, \
                 download_speed = ?, updated_at = ? WHERE id = ? AND gid = ?",
                vec![
                    status.to_string(),
                    total_length.to_string(),
                    completed_length.to_string(),
                    download_speed.to_string(),
                    Self::now_string(),
                    id.to_string(),
                    gid.to_string(),
                ],
            )
            .await?;
        ensure_row_updated(result, "下载进度")?;
        Ok(())
    }

    /// 持久化成功状态和最终文件信息。
    pub async fn mark_completed(
        &self,
        id: i64,
        gid: &str,
        final_path: &str,
        total_length: i64,
        completed_length: i64,
    ) -> Result<()> {
        let now = Self::now_string();
        let result = self
            .db
            .execute_query_with_params(
                "UPDATE sys_downloads SET status = 'completed', final_path = ?, total_length = ?, \
                 completed_length = ?, completed_at = ?, updated_at = ? WHERE id = ? AND gid = ?",
                vec![
                    final_path.to_string(),
                    total_length.to_string(),
                    completed_length.to_string(),
                    now.clone(),
                    now,
                    id.to_string(),
                    gid.to_string(),
                ],
            )
            .await?;
        ensure_row_updated(result, "完成状态")?;
        Ok(())
    }

    /// 持久化失败状态。
    pub async fn mark_failed(
        &self,
        id: i64,
        gid: &str,
        retry_count: i64,
        error: &str,
    ) -> Result<()> {
        let result = self
            .db
            .execute_query_with_params(
                "UPDATE sys_downloads SET status = 'failed', retry_count = ?, error = ?, \
                 updated_at = ? WHERE id = ? AND gid = ?",
                vec![
                    retry_count.to_string(),
                    error.to_string(),
                    Self::now_string(),
                    id.to_string(),
                    gid.to_string(),
                ],
            )
            .await?;
        ensure_row_updated(result, "失败状态")?;
        Ok(())
    }

    /// 持久化取消状态。
    pub async fn mark_cancelled(&self, id: i64, gid: &str) -> Result<()> {
        let result = self
            .db
            .execute_query_with_params(
                "UPDATE sys_downloads SET status = 'cancelled', updated_at = ? \
                 WHERE id = ? AND gid = ?",
                vec![Self::now_string(), id.to_string(), gid.to_string()],
            )
            .await?;
        ensure_row_updated(result, "取消状态")?;
        Ok(())
    }

    /// Update download status
    pub async fn update_status(&self, gid: &str, status: &str) -> Result<()> {
        self.db
            .execute_query_with_params(
                "UPDATE sys_downloads SET status = ?, updated_at = ? WHERE gid = ?",
                vec![status.to_string(), Self::now_string(), gid.to_string()],
            )
            .await?;
        Ok(())
    }

    /// Update download progress
    pub async fn update_progress(
        &self,
        gid: &str,
        total_length: i64,
        completed_length: i64,
        download_speed: i64,
    ) -> Result<()> {
        self.db
            .execute_query_with_params(
                "UPDATE sys_downloads SET total_length = ?, completed_length = ?, download_speed = ?, updated_at = ? WHERE gid = ?",
                vec![
                    total_length.to_string(),
                    completed_length.to_string(),
                    download_speed.to_string(),
                    Self::now_string(),
                    gid.to_string(),
                ],
            )
            .await?;
        Ok(())
    }

    /// Delete a download record
    pub async fn delete(&self, gid: &str) -> Result<()> {
        self.db
            .execute_query_with_params(
                "DELETE FROM sys_downloads WHERE gid = ?",
                vec![gid.to_string()],
            )
            .await?;
        Ok(())
    }

    /// List downloads, optionally filtered by status
    pub async fn list(&self, status: Option<&str>) -> Result<Vec<SysDownload>> {
        match status {
            Some(s) => {
                self.db
                    .fetch_all_with_params::<SysDownload>(
                        "SELECT id, gid, url, status, uris, total_length, completed_length, download_speed, download_dir, filename, connections, split, retry_count, error, final_path, completed_at, created_at, updated_at \
                         FROM sys_downloads WHERE status = ? ORDER BY created_at DESC",
                        vec![s.to_string()],
                    )
                    .await
            }
            None => {
                self.db
                    .fetch_all::<SysDownload>(
                        "SELECT id, gid, url, status, uris, total_length, completed_length, download_speed, download_dir, filename, connections, split, retry_count, error, final_path, completed_at, created_at, updated_at \
                         FROM sys_downloads ORDER BY created_at DESC",
                    )
                    .await
            }
        }
    }

    /// 按 GID 查询单条下载记录。
    ///
    /// 只有查询成功且 GID 不存在时才返回 `Ok(None)`；查询失败会向调用方传播，不能被误认为记录不存在。
    pub async fn get(&self, gid: &str) -> Result<Option<SysDownload>> {
        self.db
            .fetch_optional_with_params::<SysDownload>(
                "SELECT gid, status, uris, total_length, completed_length, download_speed, download_dir, filename, connections, split, created_at, updated_at \
                 FROM sys_downloads WHERE gid = ?",
                vec![gid.to_string()],
            )
            .await
    }

    /// Update the GID of a download record (used when restoring incomplete downloads)
    pub async fn update_gid(&self, old_gid: &str, new_gid: &str) -> Result<()> {
        self.db
            .execute_query_with_params(
                "UPDATE sys_downloads SET gid = ?, updated_at = ? WHERE gid = ?",
                vec![new_gid.to_string(), Self::now_string(), old_gid.to_string()],
            )
            .await?;
        Ok(())
    }
}

fn ensure_row_updated(
    result: burncloud_database::sqlx::any::AnyQueryResult,
    operation: &str,
) -> Result<()> {
    if result.rows_affected() == 0 {
        return Err(DatabaseError::Query(format!(
            "未找到需要更新的下载记录：{operation}"
        )));
    }
    Ok(())
}
