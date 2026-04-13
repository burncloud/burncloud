use burncloud_database::{Database, Result};
use serde::{Deserialize, Serialize};

#[derive(sqlx::FromRow, Serialize, Deserialize, Clone)]
pub struct Download {
    pub gid: String,
    pub status: String,
    pub uris: String,
    pub total_length: i64,
    pub completed_length: i64,
    pub download_speed: i64,
    pub download_dir: Option<String>,
    pub filename: Option<String>,
    pub connections: i32,
    pub split: i32,
    pub created_at: String,
    pub updated_at: String,
}

pub struct DownloadDB {
    db: Database,
}

impl DownloadDB {
    pub async fn new() -> Result<Self> {
        let db = Database::new().await?;
        let instance = Self { db };
        instance.init_tables().await?;
        Ok(instance)
    }

    async fn init_tables(&self) -> Result<()> {
        self.db
            .execute_query(
                "
            CREATE TABLE IF NOT EXISTS downloads (
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
                created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now')),
                updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now'))
            );
            CREATE INDEX IF NOT EXISTS idx_downloads_status ON downloads(status);
        ",
            )
            .await?;

        // Migrate DATETIME → TEXT: the sqlx Any driver cannot decode SQLite's
        // Datetime type affinity. If the table was created before this fix,
        // recreate it with TEXT columns preserving all existing rows.
        self.migrate_datetime_columns().await?;

        Ok(())
    }

    async fn migrate_datetime_columns(&self) -> Result<()> {
        // Check whether created_at was stored with DATETIME affinity.
        // PRAGMA table_info returns one row per column; type = "DATETIME" means
        // the old schema. sqlx Any can query PRAGMA as raw rows.
        let rows = self
            .db
            .query("PRAGMA table_info(downloads)")
            .await?;

        let needs_migration = rows.iter().any(|row| {
            use sqlx::Row;
            let col_name: String = row.try_get("name").unwrap_or_default();
            let col_type: String = row.try_get("type").unwrap_or_default();
            (col_name == "created_at" || col_name == "updated_at")
                && col_type.to_uppercase() == "DATETIME"
        });

        if !needs_migration {
            return Ok(());
        }

        // SQLite doesn't support ALTER COLUMN; recreate via rename-copy-drop.
        self.db.execute_query("
            BEGIN;
            ALTER TABLE downloads RENAME TO downloads_old;
            CREATE TABLE downloads (
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
                created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now')),
                updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now'))
            );
            INSERT OR IGNORE INTO downloads
                SELECT gid, status, uris, total_length, completed_length, download_speed,
                       download_dir, filename, connections, split,
                       strftime('%Y-%m-%dT%H:%M:%S', created_at),
                       strftime('%Y-%m-%dT%H:%M:%S', updated_at)
                FROM downloads_old;
            DROP TABLE downloads_old;
            CREATE INDEX IF NOT EXISTS idx_downloads_status ON downloads(status);
            COMMIT;
        ").await?;

        Ok(())
    }

    pub async fn add(
        &self,
        gid: &str,
        uris: Vec<String>,
        download_dir: Option<&str>,
        filename: Option<&str>,
    ) -> Result<()> {
        if self.get(gid).await?.is_some() {
            return Ok(());
        }

        let uris_json = serde_json::to_string(&uris)
            .map_err(burncloud_database::error::DatabaseError::Serialization)?;
        let download_dir_str = download_dir.unwrap_or("").to_string();

        // 检查相同的uris和download_dir组合是否已存在（使用参数化查询防止 SQL 注入）
        let existing = self
            .db
            .fetch_optional_with_params::<Download>(
                "SELECT * FROM downloads WHERE uris = ? AND download_dir = ?",
                vec![uris_json.clone(), download_dir_str.clone()],
            )
            .await?;

        if existing.is_some() {
            // Duplicate uris+download_dir: silently skip, keep the original row.
            return Ok(());
        }

        self.db
            .execute_query_with_params(
                "INSERT INTO downloads (gid, uris, download_dir, filename) VALUES (?, ?, ?, ?)",
                vec![
                    gid.to_string(),
                    uris_json,
                    download_dir_str,
                    filename.unwrap_or("").to_string(),
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn update_progress(
        &self,
        gid: &str,
        total: i64,
        completed: i64,
        speed: i64,
    ) -> Result<()> {
        self.db.execute_query_with_params(
            "UPDATE downloads SET total_length = ?, completed_length = ?, download_speed = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%S', 'now') WHERE gid = ?",
            vec![total.to_string(), completed.to_string(), speed.to_string(), gid.to_string()]
        ).await?;
        Ok(())
    }

    pub async fn update_status(&self, gid: &str, status: &str) -> Result<()> {
        self.db
            .execute_query_with_params(
                "UPDATE downloads SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%S', 'now') WHERE gid = ?",
                vec![status.to_string(), gid.to_string()],
            )
            .await?;
        Ok(())
    }

    pub async fn update_gid(&self, old_gid: &str, new_gid: &str) -> Result<()> {
        self.db
            .execute_query_with_params(
                "UPDATE downloads SET gid = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%S', 'now') WHERE gid = ?",
                vec![new_gid.to_string(), old_gid.to_string()],
            )
            .await?;
        Ok(())
    }

    pub async fn get(&self, gid: &str) -> Result<Option<Download>> {
        self.db
            .fetch_optional_with_params::<Download>(
                "SELECT * FROM downloads WHERE gid = ?",
                vec![gid.to_string()],
            )
            .await
    }

    pub async fn list(&self, status: Option<&str>) -> Result<Vec<Download>> {
        match status {
            Some(s) => {
                self.db
                    .fetch_all_with_params::<Download>(
                        "SELECT * FROM downloads WHERE status = ? ORDER BY created_at DESC",
                        vec![s.to_string()],
                    )
                    .await
            }
            None => {
                self.db
                    .fetch_all::<Download>("SELECT * FROM downloads ORDER BY created_at DESC")
                    .await
            }
        }
    }

    pub async fn delete(&self, gid: &str) -> Result<()> {
        self.db
            .execute_query_with_params("DELETE FROM downloads WHERE gid = ?", vec![gid.to_string()])
            .await?;
        Ok(())
    }
}
