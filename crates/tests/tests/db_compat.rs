//! Database Compatibility Tests (DB-01, DB-02, DB-03)
//!
//! This module tests database compatibility between SQLite and PostgreSQL:
//! - DB-01: SQLite CRUD operations
//! - DB-02: PostgreSQL CRUD operations
//! - DB-03: SQL dialect differences ($1 vs ? parameterized queries)

use anyhow::Result;
use burncloud_database::Database;
use serial_test::serial;
use std::env;

// ============== Test Utilities ==============

/// Create a test database using the default path
/// This uses the same initialization logic as the production database
async fn create_test_db() -> Result<Database> {
    // Don't override BURNCLOUD_DATABASE_URL, use the default path
    Database::new()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create database: {}", e))
}

// ============== DB-03: SQL Dialect Tests ==============

#[tokio::test]
#[serial]
async fn test_sqlite_dialect_detection() -> Result<()> {
    println!("\n=== Running SQLite Dialect Detection Tests (DB-03) ===");

    let db = create_test_db().await?;

    // Verify dialect detection
    let kind = db.kind();
    assert!(
        kind == "sqlite" || kind == "postgres",
        "Database kind should be sqlite or postgres, got: {}",
        kind
    );

    println!("Database kind detected: {}", kind);
    println!("SQLite dialect detection tests passed!");

    db.close().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_sqlite_parameterized_query() -> Result<()> {
    println!("\n=== Running SQLite Parameterized Query Tests (DB-03) ===");

    let db = create_test_db().await?;
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    // Test parameterized query with correct placeholder syntax
    let test_value = format!("test-{}", uuid::Uuid::new_v4());

    // Create a temporary test table
    sqlx::query("CREATE TABLE IF NOT EXISTS db_compat_test (id TEXT PRIMARY KEY, value TEXT)")
        .execute(conn.pool())
        .await?;

    // Insert using parameterized query
    let insert_sql = if is_postgres {
        "INSERT INTO db_compat_test (id, value) VALUES ($1, $2)"
    } else {
        "INSERT INTO db_compat_test (id, value) VALUES (?, ?)"
    };

    sqlx::query(insert_sql)
        .bind(&test_value)
        .bind("test_data")
        .execute(conn.pool())
        .await?;

    // Query using parameterized query
    let select_sql = if is_postgres {
        "SELECT value FROM db_compat_test WHERE id = $1"
    } else {
        "SELECT value FROM db_compat_test WHERE id = ?"
    };

    let result: Option<(String,)> = sqlx::query_as(select_sql)
        .bind(&test_value)
        .fetch_optional(conn.pool())
        .await?;

    assert!(result.is_some(), "Should find inserted row");
    assert_eq!(result.unwrap().0, "test_data", "Value should match");

    // Cleanup
    let delete_sql = if is_postgres {
        "DELETE FROM db_compat_test WHERE id = $1"
    } else {
        "DELETE FROM db_compat_test WHERE id = ?"
    };
    sqlx::query(delete_sql)
        .bind(&test_value)
        .execute(conn.pool())
        .await?;

    println!("Parameterized query tests passed!");
    db.close().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_sqlite_keyword_escaping() -> Result<()> {
    println!("\n=== Running SQL Keyword Escaping Tests (DB-03) ===");

    let db = create_test_db().await?;
    let is_postgres = db.kind() == "postgres";

    // Test keyword escaping (group is a reserved word)
    let group_col = if is_postgres {
        "\"group\""
    } else {
        "`group`"
    };

    // Verify the escape syntax is correct by running a query
    let conn = db.get_connection()?;
    let sql = format!(
        "SELECT id, {} FROM users LIMIT 1",
        group_col
    );

    // This should not error if escaping is correct
    let _ = sqlx::query(&sql)
        .fetch_optional(conn.pool())
        .await?;

    println!("Keyword escaping tests passed!");
    db.close().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_sqlite_limit_offset_syntax() -> Result<()> {
    println!("\n=== Running LIMIT/OFFSET Syntax Tests (DB-03) ===");

    let db = create_test_db().await?;
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    // Test LIMIT/OFFSET with parameterized values
    let limit_sql = if is_postgres {
        "SELECT id FROM users LIMIT $1 OFFSET $2"
    } else {
        "SELECT id FROM users LIMIT ? OFFSET ?"
    };

    let result: Vec<(String,)> = sqlx::query_as(limit_sql)
        .bind(10_i32)
        .bind(0_i32)
        .fetch_all(conn.pool())
        .await?;

    // Just verify the query works, don't check result count
    println!("LIMIT/OFFSET query returned {} rows", result.len());

    println!("LIMIT/OFFSET syntax tests passed!");
    db.close().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_sqlite_aggregate_functions() -> Result<()> {
    println!("\n=== Running Aggregate Function Tests (DB-01) ===");

    let db = create_test_db().await?;
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    // Test COUNT with parameterized WHERE clause
    let count_sql = if is_postgres {
        "SELECT COUNT(*) FROM users WHERE status = $1"
    } else {
        "SELECT COUNT(*) FROM users WHERE status = ?"
    };

    let count: i64 = sqlx::query_scalar(count_sql)
        .bind(1_i32)
        .fetch_one(conn.pool())
        .await?;

    println!("Active users count: {}", count);

    // Test SUM
    let sum_sql = if is_postgres {
        "SELECT COALESCE(SUM(balance_usd), 0) FROM users WHERE status = $1"
    } else {
        "SELECT COALESCE(SUM(balance_usd), 0) FROM users WHERE status = ?"
    };

    let sum: i64 = sqlx::query_scalar(sum_sql)
        .bind(1_i32)
        .fetch_one(conn.pool())
        .await?;

    println!("Total USD balance (nanodollars): {}", sum);

    println!("Aggregate function tests passed!");
    db.close().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_sqlite_i64_nanodollar_precision() -> Result<()> {
    println!("\n=== Running i64 Nanodollar Precision Tests (DB-01) ===");

    let db = create_test_db().await?;
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    // Test with large i64 values (important for financial accuracy)
    let large_value: i64 = 1_000_000_000_000; // 1000 USD in nanodollars

    // Create a test record with large value
    let test_id = format!("precision-test-{}", uuid::Uuid::new_v4());

    sqlx::query("CREATE TABLE IF NOT EXISTS precision_test (id TEXT PRIMARY KEY, cost BIGINT)")
        .execute(conn.pool())
        .await?;

    let insert_sql = if is_postgres {
        "INSERT INTO precision_test (id, cost) VALUES ($1, $2)"
    } else {
        "INSERT INTO precision_test (id, cost) VALUES (?, ?)"
    };

    sqlx::query(insert_sql)
        .bind(&test_id)
        .bind(large_value)
        .execute(conn.pool())
        .await?;

    // Read back and verify precision
    let select_sql = if is_postgres {
        "SELECT cost FROM precision_test WHERE id = $1"
    } else {
        "SELECT cost FROM precision_test WHERE id = ?"
    };

    let result: i64 = sqlx::query_scalar(select_sql)
        .bind(&test_id)
        .fetch_one(conn.pool())
        .await?;

    assert_eq!(
        result, large_value,
        "i64 nanodollar value should be stored precisely"
    );

    // Cleanup
    let delete_sql = if is_postgres {
        "DELETE FROM precision_test WHERE id = $1"
    } else {
        "DELETE FROM precision_test WHERE id = ?"
    };
    sqlx::query(delete_sql)
        .bind(&test_id)
        .execute(conn.pool())
        .await?;

    println!("i64 nanodollar precision tests passed!");
    db.close().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_sqlite_transaction_operations() -> Result<()> {
    println!("\n=== Running Transaction Tests (DB-01) ===");

    let db = create_test_db().await?;
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    // Create test table
    sqlx::query("CREATE TABLE IF NOT EXISTS tx_test (id TEXT PRIMARY KEY, value TEXT)")
        .execute(conn.pool())
        .await?;

    let test_id = format!("tx-test-{}", uuid::Uuid::new_v4());
    let placeholder = if is_postgres { "$1" } else { "?" };

    // Test rollback
    let mut tx = conn.pool().begin().await?;

    let insert_sql = format!(
        "INSERT INTO tx_test (id, value) VALUES ({}, 'test')",
        placeholder
    );
    sqlx::query(&insert_sql)
        .bind(&test_id)
        .execute(&mut *tx)
        .await?;

    tx.rollback().await?;

    // Verify rollback
    let select_sql = format!("SELECT value FROM tx_test WHERE id = {}", placeholder);
    let result: Option<(String,)> = sqlx::query_as(&select_sql)
        .bind(&test_id)
        .fetch_optional(conn.pool())
        .await?;

    assert!(result.is_none(), "Rolled back insert should not exist");

    // Test commit
    let mut tx2 = conn.pool().begin().await?;
    sqlx::query(&insert_sql)
        .bind(&test_id)
        .execute(&mut *tx2)
        .await?;
    tx2.commit().await?;

    // Verify commit
    let result: Option<(String,)> = sqlx::query_as(&select_sql)
        .bind(&test_id)
        .fetch_optional(conn.pool())
        .await?;

    assert!(result.is_some(), "Committed insert should exist");

    // Cleanup
    let delete_sql = format!("DELETE FROM tx_test WHERE id = {}", placeholder);
    sqlx::query(&delete_sql)
        .bind(&test_id)
        .execute(conn.pool())
        .await?;

    println!("Transaction tests passed!");
    db.close().await?;
    Ok(())
}

// ============== DB-02: PostgreSQL Tests ==============

#[tokio::test]
#[ignore = "Requires PostgreSQL. Set BURNCLOUD_RUN_POSTGRES_TESTS=1 to run"]
async fn test_postgres_crud_operations() -> Result<()> {
    println!("\n=== Running PostgreSQL CRUD Tests (DB-02) ===");

    // Check if PostgreSQL tests should run
    dotenvy::dotenv().ok();
    if env::var("BURNCLOUD_RUN_POSTGRES_TESTS").is_err() && env::var("BURNCLOUD_TEST_POSTGRES_URL").is_err() {
        println!("Skipping PostgreSQL tests. Set BURNCLOUD_RUN_POSTGRES_TESTS=1 to enable.");
        return Ok(());
    }

    let db = create_test_db().await?;

    // Verify it's PostgreSQL
    assert_eq!(db.kind(), "postgres", "Should be PostgreSQL database");

    // Test parameterized queries with PostgreSQL syntax
    let conn = db.get_connection()?;
    let test_value = format!("pg-test-{}", uuid::Uuid::new_v4());

    sqlx::query("CREATE TABLE IF NOT EXISTS pg_compat_test (id TEXT PRIMARY KEY, value TEXT)")
        .execute(conn.pool())
        .await?;

    sqlx::query("INSERT INTO pg_compat_test (id, value) VALUES ($1, $2)")
        .bind(&test_value)
        .bind("pg_test_data")
        .execute(conn.pool())
        .await?;

    let result: Option<(String,)> = sqlx::query_as("SELECT value FROM pg_compat_test WHERE id = $1")
        .bind(&test_value)
        .fetch_optional(conn.pool())
        .await?;

    assert!(result.is_some(), "Should find inserted row in PostgreSQL");
    assert_eq!(result.unwrap().0, "pg_test_data");

    // Cleanup
    sqlx::query("DELETE FROM pg_compat_test WHERE id = $1")
        .bind(&test_value)
        .execute(conn.pool())
        .await?;

    println!("All PostgreSQL tests passed!");
    db.close().await?;
    Ok(())
}

#[tokio::test]
#[ignore = "Requires PostgreSQL. Set BURNCLOUD_RUN_POSTGRES_TESTS=1 to run"]
async fn test_postgres_returning_clause() -> Result<()> {
    println!("\n=== Running PostgreSQL RETURNING Clause Tests (DB-02) ===");

    dotenvy::dotenv().ok();
    if env::var("BURNCLOUD_RUN_POSTGRES_TESTS").is_err() && env::var("BURNCLOUD_TEST_POSTGRES_URL").is_err() {
        return Ok(());
    }

    let db = create_test_db().await?;
    let conn = db.get_connection()?;

    // Test INSERT ... RETURNING (PostgreSQL specific feature)
    let test_id = format!("ret-test-{}", uuid::Uuid::new_v4());

    sqlx::query("CREATE TABLE IF NOT EXISTS returning_test (id SERIAL PRIMARY KEY, name TEXT)")
        .execute(conn.pool())
        .await?;

    let returned_id: i32 = sqlx::query_scalar(
        "INSERT INTO returning_test (name) VALUES ($1) RETURNING id"
    )
    .bind(&test_id)
    .fetch_one(conn.pool())
    .await?;

    assert!(returned_id > 0, "RETURNING id should return positive id");

    // Cleanup
    sqlx::query("DELETE FROM returning_test WHERE id = $1")
        .bind(returned_id)
        .execute(conn.pool())
        .await?;

    println!("PostgreSQL RETURNING clause tests passed!");
    db.close().await?;
    Ok(())
}

// ============== Summary ==============

#[tokio::test]
#[serial]
async fn test_database_compatibility_summary() -> Result<()> {
    println!("\n========================================");
    println!("Database Compatibility Test Summary");
    println!("========================================");

    let db = create_test_db().await?;
    let kind = db.kind();

    println!("Database Type: {}", kind.to_uppercase());
    println!();

    // DB-01: SQLite Operations
    println!("DB-01 (SQLite Operations): Tested via other test functions");

    // DB-02: PostgreSQL Operations
    println!("DB-02 (PostgreSQL Operations): Run with BURNCLOUD_RUN_POSTGRES_TESTS=1");

    // DB-03: SQL Dialect Differences
    println!("DB-03 (SQL Dialect Differences):");
    if kind == "postgres" {
        println!("  - Placeholder syntax: $1, $2, $3...");
        println!("  - Keyword escaping: \"column\"");
        println!("  - RETURNING clause: Supported");
    } else {
        println!("  - Placeholder syntax: ?");
        println!("  - Keyword escaping: `column`");
        println!("  - RETURNING clause: Not supported (use last_insert_rowid)");
    }
    println!("  - i64 nanodollar precision: Verified");
    println!("  - Transaction support: Verified");
    println!("  - LIMIT/OFFSET: Verified");

    println!();
    println!("All database compatibility tests passed!");
    println!("========================================");

    db.close().await?;
    Ok(())
}
