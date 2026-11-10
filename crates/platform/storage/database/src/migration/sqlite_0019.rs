//! SQLite 迁移 0019 为下载记录增加稳定主键和生命周期字段。

use crate::{DatabaseError, Result};
use sqlx::AnyPool;

pub(super) async fn apply(pool: &AnyPool) -> Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"CREATE TABLE sys_downloads_0019 (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            gid TEXT NOT NULL UNIQUE,
            url TEXT,
            status TEXT NOT NULL DEFAULT 'waiting',
            uris TEXT NOT NULL,
            total_length INTEGER DEFAULT 0,
            completed_length INTEGER DEFAULT 0,
            download_speed INTEGER DEFAULT 0,
            download_dir TEXT,
            filename TEXT,
            connections INTEGER DEFAULT 16,
            split INTEGER DEFAULT 5,
            retry_count INTEGER NOT NULL DEFAULT 0,
            error TEXT,
            final_path TEXT,
            completed_at TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(&mut *tx)
    .await
    .map_err(|error| DatabaseError::Migration(format!("create sys_downloads_0019: {error}")))?;

    sqlx::query(
        r#"INSERT INTO sys_downloads_0019 (
            id, gid, status, uris, total_length, completed_length, download_speed,
            download_dir, filename, connections, split, created_at, updated_at
        )
        SELECT rowid, gid, status, uris, total_length, completed_length, download_speed,
               download_dir, filename, connections, split, created_at, updated_at
        FROM sys_downloads"#,
    )
    .execute(&mut *tx)
    .await
    .map_err(|error| DatabaseError::Migration(format!("copy sys_downloads: {error}")))?;

    sqlx::query("DROP TABLE sys_downloads")
        .execute(&mut *tx)
        .await
        .map_err(|error| DatabaseError::Migration(format!("drop old sys_downloads: {error}")))?;

    sqlx::query("ALTER TABLE sys_downloads_0019 RENAME TO sys_downloads")
        .execute(&mut *tx)
        .await
        .map_err(|error| DatabaseError::Migration(format!("rename sys_downloads: {error}")))?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sys_downloads_status ON sys_downloads(status)")
        .execute(&mut *tx)
        .await
        .map_err(|error| {
            DatabaseError::Migration(format!("index sys_downloads.status: {error}"))
        })?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_sys_downloads_created_at ON sys_downloads(created_at)",
    )
    .execute(&mut *tx)
    .await
    .map_err(|error| {
        DatabaseError::Migration(format!("index sys_downloads.created_at: {error}"))
    })?;

    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "测试断言需要明确报告测试数据库的准备失败"
)]
mod tests {
    use super::apply;
    use sqlx::any::AnyPoolOptions;

    async fn memory_pool() -> sqlx::AnyPool {
        sqlx::any::install_default_drivers();
        AnyPoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:?cache=shared")
            .await
            .expect("in-memory SQLite pool")
    }

    #[tokio::test]
    async fn migration_preserves_rows_and_assigns_stable_ids() {
        let pool = memory_pool().await;
        sqlx::query(
            r#"CREATE TABLE sys_downloads (
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
            )"#,
        )
        .execute(&pool)
        .await
        .expect("create legacy table");
        sqlx::query(
            "INSERT INTO sys_downloads (gid, status, uris, download_dir, filename) \
             VALUES ('old-gid', 'active', '[\"https://example.invalid/file\"]', '/downloads', 'file')",
        )
        .execute(&pool)
        .await
        .expect("insert legacy row");

        apply(&pool).await.expect("apply migration");

        let row = sqlx::query("SELECT id, gid, status, uris FROM sys_downloads")
            .fetch_one(&pool)
            .await
            .expect("read migrated row");
        use sqlx::Row;
        assert_eq!(row.try_get::<i64, _>("id").expect("stable ID"), 1);
        assert_eq!(row.try_get::<String, _>("gid").expect("GID"), "old-gid");
        assert_eq!(
            row.try_get::<String, _>("status").expect("status"),
            "active"
        );
        assert_eq!(
            row.try_get::<String, _>("uris").expect("URIs"),
            "[\"https://example.invalid/file\"]"
        );
    }
}
