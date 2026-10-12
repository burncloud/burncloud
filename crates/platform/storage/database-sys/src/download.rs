//! Database persistence for download module

use burncloud_database::{Database, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Download task record
#[derive(sqlx::FromRow, Serialize, Deserialize, Clone)]
pub struct SysDownload {
    /// Download GID (from aria2)
    pub gid: String,
    /// Download status
    pub status: String,
    /// JSON array of URIs
    pub uris: String,
    /// Total bytes
    pub total_length: Option<i64>,
    /// Completed bytes
    pub completed_length: Option<i64>,
    /// Download speed (bytes/sec)
    pub download_speed: Option<i64>,
    /// Download directory
    pub download_dir: Option<String>,
    /// Extracted filename
    pub filename: Option<String>,
    /// Number of connections
    pub connections: Option<i64>,
    /// Number of split connections
    pub split: Option<i64>,
    /// Created timestamp
    pub created_at: Option<String>,
    /// Updated timestamp
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
        let instance = Self { db };
        instance.init_tables().await?;
        Ok(instance)
    }

    /// Wrap an existing database connection and create the download table on it.
    ///
    /// The counterpart of [`SettingDatabase::new_with_db`](crate::SettingDatabase::new_with_db), and it exists
    /// for the same reason: [`DownloadDB::new`] opens `Database::new()`, which honours `BURNCLOUD_DATABASE_URL`
    /// and otherwise uses the **shared default path** under the user's data directory. A test that used it would
    /// write download records into the developer's real database and could not run twice cleanly. This lets a
    /// caller -- a test, or a binary that already holds a connection -- supply the database instead.
    pub async fn new_with_db(db: Database) -> Result<Self> {
        let instance = Self { db };
        instance.init_tables().await?;
        Ok(instance)
    }

    /// Initialize database tables
    async fn init_tables(&self) -> Result<()> {
        self.db
            .execute_query(
                r#"
            CREATE TABLE IF NOT EXISTS sys_downloads (
                gid TEXT PRIMARY KEY,
                status TEXT NOT NULL DEFAULT 'waiting',
                uris TEXT NOT NULL,
                total_length INTEGER DEFAULT 0,
                completed_length INTEGER DEFAULT 0,
                download_speed INTEGER DEFAULT 0,
                download_dir TEXT,
                filename TEXT,
                connections INTEGER DEFAULT 16,
                split INTEGER DEFAULT 5,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_sys_downloads_status ON sys_downloads(status);
            CREATE INDEX IF NOT EXISTS idx_sys_downloads_created_at ON sys_downloads(created_at);
        "#,
            )
            .await?;
        Ok(())
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
        self.db
            .execute_query_with_params(
                r#"
                INSERT INTO sys_downloads (gid, status, uris, download_dir, filename, updated_at)
                VALUES (?, 'active', ?, ?, ?, ?)
                "#,
                vec![
                    gid.to_string(),
                    serde_json::to_string(&uris)?,
                    download_dir.unwrap_or("").to_string(),
                    filename.unwrap_or("").to_string(),
                    Self::now_string(),
                ],
            )
            .await?;
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
                        "SELECT gid, status, uris, total_length, completed_length, download_speed, download_dir, filename, connections, split, created_at, updated_at \
                         FROM sys_downloads WHERE status = ? ORDER BY created_at DESC",
                        vec![s.to_string()],
                    )
                    .await
            }
            None => {
                self.db
                    .fetch_all::<SysDownload>(
                        "SELECT gid, status, uris, total_length, completed_length, download_speed, download_dir, filename, connections, split, created_at, updated_at \
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
