//! Contract boundary for #845: infrastructure fixups and legacy seed behavior.
use burncloud_database::{
    create_database_with_url, create_infrastructure_database_with_url, sqlx,
};
use std::error::Error;

async fn count(db: &burncloud_database::Database, table: &str, predicate: &str) -> Result<i64, Box<dyn Error>> {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE {predicate}");
    Ok(sqlx::query_scalar(&sql)
        .fetch_one(db.get_connection()?.pool())
        .await?)
}

#[tokio::test]
async fn infrastructure_bootstrap_is_seed_free_and_legacy_factory_is_compatible(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("init-boundary.sqlite");
    let url = format!("sqlite:///{}?mode=rwc", path.to_string_lossy().replace('\\', "/"));

    let infrastructure = create_infrastructure_database_with_url(&url).await?;
    assert_eq!(count(&infrastructure, "user_accounts", "id = 'demo-user'").await?, 0);
    assert_eq!(count(&infrastructure, "user_api_keys", "key = 'sk-burncloud-demo'").await?, 0);
    assert_eq!(count(&infrastructure, "channel_protocol_configs", "1 = 1").await?, 0);
    infrastructure.close().await?;

    // Preserve the public factory's original demo user/token/protocol contract.
    let legacy = create_database_with_url(&url).await?;
    assert_eq!(count(&legacy, "user_accounts", "id = 'demo-user'").await?, 1);
    assert_eq!(count(&legacy, "user_api_keys", "key = 'sk-burncloud-demo'").await?, 1);
    assert_eq!(count(&legacy, "channel_protocol_configs", "1 = 1").await?, 4);
    legacy.close().await?;

    // Neither the historical factory nor the infrastructure-only factory may
    // duplicate seed rows or overwrite records on a reopened database.
    let reopened = create_infrastructure_database_with_url(&url).await?;
    assert_eq!(count(&reopened, "user_accounts", "id = 'demo-user'").await?, 1);
    assert_eq!(count(&reopened, "user_api_keys", "key = 'sk-burncloud-demo'").await?, 1);
    assert_eq!(count(&reopened, "channel_protocol_configs", "1 = 1").await?, 4);
    reopened.close().await?;

    let legacy_again = create_database_with_url(&url).await?;
    assert_eq!(count(&legacy_again, "user_accounts", "id = 'demo-user'").await?, 1);
    assert_eq!(count(&legacy_again, "user_api_keys", "key = 'sk-burncloud-demo'").await?, 1);
    assert_eq!(count(&legacy_again, "channel_protocol_configs", "1 = 1").await?, 4);
    legacy_again.close().await?;
    Ok(())
}
