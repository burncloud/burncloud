use burncloud_database::{create_database_with_url, Database};
use burncloud_database_router::RouterDatabase;
use burncloud_database_user::UserDatabase;
use std::sync::Arc;

/// Create an isolated temp-file database for a server test.
/// Each call returns a fresh `Arc<Database>` backed by a unique temp file,
/// so concurrent server tests do not share SQLite state and cannot lock each other.
pub async fn make_isolated_db() -> Arc<Database> {
    if std::env::var("MASTER_KEY").is_err() {
        std::env::set_var(
            "MASTER_KEY",
            "a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8",
        );
    }
    let tmp = tempfile::NamedTempFile::new().expect("create temp db file");
    let path = tmp.path().to_string_lossy().to_string();
    std::mem::forget(tmp); // keep file alive; OS cleans up on exit
    let url = format!("sqlite:{}", path);
    let db = create_database_with_url(&url).await.expect("open temp db");
    RouterDatabase::init(&db).await.expect("router db init");
    UserDatabase::init(&db).await.expect("user db init");
    Arc::new(db)
}
