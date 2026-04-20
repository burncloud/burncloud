//! Database persistence for download module

use burncloud_database::{Database, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Download task record
#[derive(sqlx::FromRow, Serialize, Deserialize, Clone)]
pub struct SysDownload {
    /// Download GID (from aria2)
    pub gid: String,
    /// JSON array of URIs
    pub uris: String,
    /// Download directory
    pub download_dir: Option<String>,
    /// Extracted filename
    pub filename: Option<String>,
    /// Download status
    pub status: String,
    /// Total bytes
    pub total: i64,
    /// Completed bytes
    pub completed: i64,
    /// Download speed (bytes/sec)
    pub speed: i64,
    /// Created timestamp
    pub created_at: String,
    /// Updated timestamp
    pub updated_at: String,
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

    /// Initialize database tables
    async fn init_tables(&self) -> Result<()> {
        self.db
            .execute_query(
                r#"
            CREATE TABLE IF NOT EXISTS sys_downloads (
                gid TEXT PRIMARY KEY,
                uris TEXT NOT NULL,
                download_dir TEXT,
                filename TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                total INTEGER NOT NULL DEFAULT 0,
                completed INTEGER NOT NULL DEFAULT 0,
                speed INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
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
                INSERT INTO sys_downloads (gid, uris, download_dir, filename, status, updated_at)
                VALUES (?, ?, ?, ?, 'active', ?)
                "#,
                vec![
                    gid.to_string(),
                    serde_json::to_string(&uris).unwrap_or_default(),
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
        total: i64,
        completed: i64,
        speed: i64,
    ) -> Result<()> {
        self.db
            .execute_query_with_params(
                "UPDATE sys_downloads SET total = ?, completed = ?, speed = ?, updated_at = ? WHERE gid = ?",
                vec![
                    total.to_string(),
                    completed.to_string(),
                    speed.to_string(),
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
                        "SELECT * FROM sys_downloads WHERE status = ? ORDER BY created_at DESC",
                        vec![s.to_string()],
                    )
                    .await
            }
            None => {
                self.db
                    .fetch_all::<SysDownload>(
                        "SELECT * FROM sys_downloads ORDER BY created_at DESC",
                    )
                    .await
            }
        }
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
