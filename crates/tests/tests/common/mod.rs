use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::path::PathBuf;

pub async fn get_db_pool() -> Pool<Sqlite> {
    // Windows specific logic for now, matching burncloud-database
    let user_profile = std::env::var("USERPROFILE").unwrap();
    let db_path = PathBuf::from(user_profile)
        .join("AppData/Local/BurnCloud/data.db");
    
    // Ensure path uses forward slashes for URL
    let url = format!("sqlite:///{}?mode=rwc", db_path.to_string_lossy().replace('\\', "/"));
    
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("Failed to connect to test DB")
}

pub async fn seed_demo_data(mock_url: &str) {
    let pool = get_db_pool().await;
    
    // Clean up old test data
    let _ = sqlx::query("DELETE FROM abilities WHERE channel_id = 999").execute(&pool).await;
    let _ = sqlx::query("DELETE FROM channels WHERE id = 999").execute(&pool).await;

    // Insert Channel pointing to Mock Server
    // Note: `group` is a keyword
    sqlx::query(r#"
        INSERT INTO channels (id, type, key, status, name, base_url, models, `group`, priority)
        VALUES (999, 1, 'sk-mock-key', 1, 'Test Mock Channel', ?, 'gpt-3.5-turbo', 'default', 100)
    "#)
    .bind(mock_url)
    .execute(&pool).await.unwrap();

    // Insert Ability
    sqlx::query(r#"
        INSERT INTO abilities (`group`, model, channel_id, enabled, priority, weight)
        VALUES ('default', 'gpt-3.5-turbo', 999, 1, 100, 10)
    "#)
    .execute(&pool).await.unwrap();
    
    println!("Seeded DB with Mock Channel -> {}", mock_url);
}
