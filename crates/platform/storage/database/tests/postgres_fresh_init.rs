//! PostgreSQL fresh-install and repeat-initialization regression for #833.
//! Only executes against an explicitly provided disposable PostgreSQL server.
//! Never points this test at a production instance: it creates and drops a DB.

use burncloud_database::create_database_with_url;
use burncloud_database::sqlx::{self, ConnectOptions, Executor};
use std::{error::Error, str::FromStr};

const ENV: &str = "BURNCLOUD_TEST_POSTGRES_URL";

#[tokio::test]
async fn postgres_fresh_install_and_reopen_preserve_schema_and_demo_seed(
) -> Result<(), Box<dyn Error>> {
    let Ok(server_url) = std::env::var(ENV) else {
        println!("SKIPPED: {ENV} unset; PostgreSQL fresh-install contract NOT exercised");
        return Ok(());
    };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let name = format!("bc_pg_833_{}_{}", std::process::id(), stamp);
    let opts = sqlx::postgres::PgConnectOptions::from_str(&server_url)?;
    let mut admin = opts.connect().await?;
    admin
        .execute(format!("CREATE DATABASE {name}").as_str())
        .await?;

    let without_query = server_url.split('?').next().unwrap_or(&server_url);
    let prefix = without_query
        .rsplit_once('/')
        .map(|(p, _)| p)
        .ok_or("PostgreSQL URL must include the database name")?;
    let target_url = format!("{prefix}/{name}");
    let db = create_database_with_url(&target_url).await?;

    let conn = db.get_connection()?;
    let user_type: String = sqlx::query_scalar(
        "SELECT data_type FROM information_schema.columns \
         WHERE table_name = 'billing_subscriptions' AND column_name = 'user_id'",
    )
    .fetch_one(conn.pool())
    .await?;
    assert_eq!(user_type, "text", "subscription FK must match users.id");

    let seeded: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_accounts WHERE id = $1")
        .bind("demo-user")
        .fetch_one(conn.pool())
        .await?;
    assert_eq!(seeded, 1, "demo user must seed on real PostgreSQL");

    let request_log_fks: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_constraint \
         WHERE conrelid = 'router_request_logs'::regclass AND contype = 'f'",
    )
    .fetch_one(conn.pool())
    .await?;
    assert_eq!(
        request_log_fks, 0,
        "request_id cannot FK to router_logs, because request_id is non-unique"
    );

    db.close().await?;
    let reopened = create_database_with_url(&target_url).await?;
    let seeded_again: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_accounts WHERE id = $1")
        .bind("demo-user")
        .fetch_one(reopened.get_connection()?.pool())
        .await?;
    assert_eq!(
        seeded_again, 1,
        "reinitialization must not duplicate seed data"
    );
    reopened.close().await?;

    admin
        .execute(format!("DROP DATABASE {name} WITH (FORCE)").as_str())
        .await?;
    Ok(())
}
