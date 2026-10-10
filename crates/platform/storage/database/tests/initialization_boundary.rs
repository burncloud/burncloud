//! Contract boundary for #845: infrastructure fixups and legacy seed behavior.
use burncloud_database::{create_database_with_url, create_infrastructure_database_with_url, sqlx};
use std::error::Error;

async fn count(
    db: &burncloud_database::Database,
    table: &str,
    predicate: &str,
) -> Result<i64, Box<dyn Error>> {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE {predicate}");
    Ok(sqlx::query_scalar(&sql)
        .fetch_one(db.get_connection()?.pool())
        .await?)
}

async fn assert_seeds(
    db: &burncloud_database::Database,
    expected_users: i64,
    expected_tokens: i64,
    expected_protocols: i64,
) -> Result<(), Box<dyn Error>> {
    let users = count(db, "user_accounts", "id = 'demo-user'").await?;
    let tokens = count(db, "user_api_keys", "key = 'sk-burncloud-demo'").await?;
    let protocols = count(db, "channel_protocol_configs", "1 = 1").await?;
    assert_eq!(users, expected_users);
    assert_eq!(tokens, expected_tokens);
    assert_eq!(protocols, expected_protocols);
    Ok(())
}

#[tokio::test]
async fn infrastructure_bootstrap_is_seed_free_and_legacy_factory_is_compatible(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("init-boundary.sqlite");
    let url = format!(
        "sqlite:///{}?mode=rwc",
        path.to_string_lossy().replace('\\', "/")
    );

    let infrastructure = create_infrastructure_database_with_url(&url).await?;
    assert_seeds(&infrastructure, 0, 0, 0).await?;
    infrastructure.close().await?;

    let legacy = create_database_with_url(&url).await?;
    assert_seeds(&legacy, 1, 1, 4).await?;
    legacy.close().await?;

    // Reinitialization must not remove, duplicate or overwrite seeded data.
    let reopened = create_infrastructure_database_with_url(&url).await?;
    assert_seeds(&reopened, 1, 1, 4).await?;
    reopened.close().await?;

    let legacy_again = create_database_with_url(&url).await?;
    assert_seeds(&legacy_again, 1, 1, 4).await?;
    legacy_again.close().await?;
    Ok(())
}
