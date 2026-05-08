#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types)]
use burncloud_database::create_database_with_url;
use std::fs;

/// Create an isolated SQLite database in a temp file to avoid shared-state races
/// when tests run in parallel. Each call produces a unique file via PID + label.
pub async fn create_isolated_db(label: &str) -> burncloud_database::Result<burncloud_database::Database> {
    let temp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let db_path = temp_dir.join(format!("burncloud_test_{}_{}.db", pid, label));
    // Clean up any leftover from a previous run
    let _ = fs::remove_file(&db_path);
    let _ = fs::remove_file(db_path.with_extension("db-wal"));
    let _ = fs::remove_file(db_path.with_extension("db-shm"));
    let normalized = db_path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{}?mode=rwc", normalized);
    create_database_with_url(&url).await
}
