use burncloud_database::{create_default_database, sqlx};
use burncloud_database_router::RouterDatabase;
use sqlx::Row;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;
    let conn = db.connection()?;

    println!("🔍 Searching for existing Bedrock configuration...");

    let row = sqlx::query(
        "SELECT base_url FROM router_upstreams WHERE id LIKE '%bedrock%' OR base_url LIKE '%amazonaws%'"
    )
    .fetch_optional(conn.pool())
    .await?;

    let base_url = match row {
        Some(r) => {
            let url: String = r.get("base_url");
            println!("✅ Found existing Base URL: {}", url);
            url
        }
        None => {
            let default = "https://bedrock-runtime.us-east-1.amazonaws.com".to_string();
            println!("⚠️ No existing config found. Using default: {}", default);
            default
        }
    };

    let id = "test-aws-apikey";
    let name = "AWS API Key Test";
    let api_key = "YOUR_API_KEY_HERE"; // Placeholder
    let match_path = "/aws-key-test";
    let auth_type = "Header:x-api-key";
    let priority = 5;

    println!(
        "💾 Inserting configuration for '{}' with priority {}...",
        id, priority
    );

    sqlx::query(
        r#"
        INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type, priority)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET 
            api_key = excluded.api_key,
            base_url = excluded.base_url,
            auth_type = excluded.auth_type,
            priority = excluded.priority
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(&base_url)
    .bind(api_key)
    .bind(match_path)
    .bind(auth_type)
    .bind(priority)
    .execute(conn.pool())
    .await?;

    println!("✅ Configuration saved! You can now run the test.");
    Ok(())
}
