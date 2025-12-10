use burncloud_database::{create_default_database, sqlx, Database};
use burncloud_database_router::RouterDatabase;
use sqlx::AnyPool;
use std::sync::Arc;
use tokio::net::TcpListener;

pub async fn setup_db() -> anyhow::Result<(Database, AnyPool)> {
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;
    let conn = db.get_connection()?;
    let pool = conn.pool().clone();
    Ok((db, pool))
}

pub async fn start_test_server(port: u16) {
    // We create a new DB instance pointing to the same file
    // Note: Tests run sequentially or use different ports/tables?
    // They use the same default DB file. WAL mode handles concurrency.
    let db = create_default_database().await.expect("Failed to open DB");
    let db_arc = Arc::new(db);

    let app = burncloud_router::create_router_app(db_arc).await.expect("Failed to create app");

    tokio::spawn(async move {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });
    // Give server a moment to start
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
}