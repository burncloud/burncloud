//! PostgreSQL fresh-install and repeat-initialization regression for #833.
//! Only executes against an explicitly provided disposable PostgreSQL server.
//! Never points this test at a production instance: it creates and drops a DB.
//!
//! Scope after #842: this test owns the **schema and re-initialization** contract on real
//! PostgreSQL — that a fresh install produces the expected schema, that an existing row
//! survives a repeat initialization, and that the factory itself writes no domain default
//! records. The demo-account seed moved to `identity/user`, and this crate may not depend on
//! that owner (`deny.toml` bans `burncloud-supply-channel` from `burncloud-database`, which is
//! the same one-way direction), so the owner-seeded startup contract — including its
//! idempotence across a reopen — is asserted in
//! `crates/interfaces/server/tests/owner_seed_contract.rs`, where `burncloud-server` is an
//! allowed consumer.

use burncloud_database::create_database_with_url;
use burncloud_database::sqlx::{self, ConnectOptions, Executor};
use std::{error::Error, str::FromStr};

const ENV: &str = "BURNCLOUD_TEST_POSTGRES_URL";

/// Compare and fail with a described error rather than panicking: this helper is used from
/// `Result`-returning functions, and `clippy::panic_in_result_fn` is in this workspace's lint
/// set.
fn ensure_eq<T: std::fmt::Debug + PartialEq>(
    actual: T,
    expected: T,
    context: &str,
) -> Result<(), Box<dyn Error>> {
    if actual == expected {
        Ok(())
    } else {
        Err(
            std::io::Error::other(format!("{context}: expected {expected:?}, got {actual:?}"))
                .into(),
        )
    }
}

#[tokio::test]
async fn postgres_fresh_install_and_reopen_preserve_schema_without_domain_seeds(
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
    ensure_eq(
        user_type.as_str(),
        "text",
        "subscription FK must match users.id",
    )?;

    // #842: the factory no longer writes domain default records, so the demo account is
    // absent until its Identity owner is asked to seed it.
    let seeded: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_accounts WHERE id = $1")
        .bind("demo-user")
        .fetch_one(conn.pool())
        .await?;
    ensure_eq(
        seeded,
        0_i64,
        "the database factory must not create the demo account (#842)",
    )?;

    // A pre-existing row must survive a repeat initialization rather than be reset.
    sqlx::query(
        "INSERT INTO user_accounts (id, username, password_hash, status) \
         VALUES ('pg-kept-user', 'pg-kept-user', 'no-login', 1)",
    )
    .execute(conn.pool())
    .await?;

    let request_log_fks: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_constraint \
         WHERE conrelid = 'router_request_logs'::regclass AND contype = 'f'",
    )
    .fetch_one(conn.pool())
    .await?;
    ensure_eq(
        request_log_fks,
        0_i64,
        "request_id cannot FK to router_logs, because request_id is non-unique",
    )?;

    db.close().await?;
    let reopened = create_database_with_url(&target_url).await?;
    let kept: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_accounts WHERE id = $1")
        .bind("pg-kept-user")
        .fetch_one(reopened.get_connection()?.pool())
        .await?;
    ensure_eq(
        kept,
        1_i64,
        "reinitialization must not drop an existing account",
    )?;
    let still_unseeded: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_accounts WHERE id = $1")
            .bind("demo-user")
            .fetch_one(reopened.get_connection()?.pool())
            .await?;
    ensure_eq(
        still_unseeded,
        0_i64,
        "reinitialization must not create the demo account either (#842)",
    )?;
    reopened.close().await?;

    admin
        .execute(format!("DROP DATABASE {name} WITH (FORCE)").as_str())
        .await?;
    Ok(())
}
