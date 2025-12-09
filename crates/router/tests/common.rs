use burncloud_database::{create_default_database, sqlx, Database};
use burncloud_database_router::RouterDatabase;
use sqlx::Pool;
use sqlx::Sqlite;

pub async fn setup_db() -> anyhow::Result<(Database, Pool<Sqlite>)> {
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;
    let conn = db.connection()?;
    // Clone the pool properly
    let pool = conn.pool().clone();
    Ok((db, pool))
}

pub async fn start_test_server(port: u16) {
    tokio::spawn(async move {
        if let Err(e) = burncloud_router::start_server(port).await {
            eprintln!("Server error on port {}: {}", port, e);
        }
    });
    // Give server a moment to start
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
}
