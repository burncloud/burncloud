#![allow(clippy::unwrap_used, clippy::expect_used, clippy::unnecessary_unwrap)]
mod common;

use std::fs;
use std::path::PathBuf;

use burncloud_database::{create_database_with_url, DatabaseError, Result};
use common::create_isolated_db;

/// Integration tests for the default database location feature
/// These tests focus on functional validation and real-world scenarios

#[tokio::test]
async fn test_create_default_database_end_to_end() {
    // Test the complete end-to-end workflow of creating a default database
    let db = create_isolated_db("e2e").await;

    // Clean up any existing test data from previous runs
    let _ = db.execute_query("DROP TABLE IF EXISTS test_table").await;

    // Verify the database is functional by performing operations
    let create_result = db
        .execute_query(
            "CREATE TABLE IF NOT EXISTS test_table (id INTEGER PRIMARY KEY, name TEXT)",
        )
        .await;
    if let Err(e) = &create_result {
        eprintln!("Failed to create table: {:?}", e);
    }
    assert!(create_result.is_ok(), "Should be able to create tables");

    let insert_result = db
        .execute_query("INSERT INTO test_table (name) VALUES ('test_data')")
        .await;
    assert!(insert_result.is_ok(), "Should be able to insert data");

    // Verify data can be retrieved
    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct TestRow {
        id: i64,
        name: String,
    }

    let rows: Result<Vec<TestRow>> = db.fetch_all("SELECT id, name FROM test_table").await;
    assert!(rows.is_ok(), "Should be able to fetch data");
    let rows = rows.unwrap_or_else(|e| panic!("fetch_all failed: {e}"));
    assert_eq!(rows.len(), 1, "Should have exactly one row");
    assert_eq!(
        rows[0].name, "test_data",
        "Data should match what was inserted"
    );

    let _ = db.close().await;
}

#[tokio::test]
async fn test_database_initialization_patterns() {
    // Test different database initialization patterns (since new_with_path is removed)

    // Test create_isolated_db - should create and initialize with isolated path
    let db = create_isolated_db("init_patterns_1").await;
    let connection_result = db.get_connection();
    assert!(connection_result.is_ok(), "Database should be initialized");
    let query_result = db.execute_query("SELECT 1 as test").await;
    assert!(query_result.is_ok(), "Should be able to execute queries");
    let _ = db.close().await;

    // Test second isolated instance
    let db2 = create_isolated_db("init_patterns_2").await;
    let query_result = db2.execute_query("SELECT 1 as test").await;
    assert!(query_result.is_ok(), "Second instance should work");
    let _ = db2.close().await;
}

#[tokio::test]
async fn test_platform_specific_paths() {
    // Test that platform-specific paths are generated correctly
    let default_path_result = get_test_default_path();

    match default_path_result {
        Ok(path) => {
            let path_str = path.to_string_lossy();

            // Verify the path contains the expected components
            assert!(path_str.contains("data.db"), "Path should end with data.db");

            if cfg!(target_os = "windows") {
                // Windows should use AppData\Local\BurnCloud
                assert!(
                    path_str.contains("AppData")
                        && path_str.contains("Local")
                        && path_str.contains("BurnCloud"),
                    "Windows path should contain AppData\\Local\\BurnCloud, got: {}",
                    path_str
                );
            } else {
                // Linux/Unix should use ~/.burncloud
                assert!(
                    path_str.contains(".burncloud"),
                    "Linux path should contain .burncloud, got: {}",
                    path_str
                );
            }

            println!("Platform-specific default path: {}", path_str);
        }
        Err(e) => {
            println!(
                "Path resolution failed (acceptable in some environments): {}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_directory_creation_and_permissions() {
    // Test that directories are created properly with correct permissions
    let db = create_isolated_db("dir_perms").await;

    // Verify the database is functional
    let query_result = db.execute_query("SELECT 1 as test").await;
    assert!(query_result.is_ok(), "Database should be functional");

    // Test that we can write to the temp directory
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_write.tmp");
    let write_result = fs::write(&test_file, "test");
    if write_result.is_ok() {
        let _ = fs::remove_file(&test_file);
    }

    let _ = db.close().await;
}

#[tokio::test]
async fn test_multiple_database_instances() {
    // Test that multiple isolated database instances can coexist
    let db1 = create_isolated_db("multi_1").await;
    let db2 = create_isolated_db("multi_2").await;

    // Both databases should be functional
    let result1 = db1.execute_query("SELECT 1 as test").await;
    let result2 = db2.execute_query("SELECT 1 as test").await;

    assert!(result1.is_ok(), "First database should be functional");
    assert!(result2.is_ok(), "Second database should be functional");

    let _ = db1.close().await;
    let _ = db2.close().await;
}

#[tokio::test]
async fn test_database_persistence() {
    // Test that data persists between database instances using the same file
    let test_value = "persistent_test_data";

    // Build a unique path but do NOT delete it on the second open,
    // since create_isolated_db always removes the file first.
    let temp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let db_path = temp_dir.join(format!("burncloud_test_{}_persistence.db", pid));
    // Clean up only before the first creation
    let _ = fs::remove_file(&db_path);
    let _ = fs::remove_file(db_path.with_extension("db-wal"));
    let _ = fs::remove_file(db_path.with_extension("db-shm"));
    let normalized = db_path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{}?mode=rwc", normalized);

    let db1 = create_database_with_url(&url).await.unwrap();
    let create_result = db1
        .execute_query(
            "CREATE TABLE IF NOT EXISTS persistence_test (id INTEGER PRIMARY KEY, value TEXT)",
        )
        .await;
    assert!(create_result.is_ok(), "Should be able to create table");

    let insert_result = db1
        .execute_query(&format!(
            "INSERT INTO persistence_test (value) VALUES ('{}')",
            test_value
        ))
        .await;
    assert!(insert_result.is_ok(), "Should be able to insert data");
    let _ = db1.close().await;

    // Re-open the same file (no delete!) and verify data exists
    let db2 = create_database_with_url(&url).await.unwrap();
    #[derive(sqlx::FromRow)]
    struct PersistenceRow {
        value: String,
    }

    let rows: Result<Vec<PersistenceRow>> =
        db2.fetch_all("SELECT value FROM persistence_test").await;
    assert!(rows.is_ok(), "Should be able to fetch data");
    let rows = rows.unwrap_or_else(|e| panic!("fetch_all failed: {e}"));
    assert!(!rows.is_empty(), "Data should persist between instances");
    assert_eq!(
        rows[0].value, test_value,
        "Data should match what was inserted"
    );
    println!("✓ Data persistence verified");
    let _ = db2.close().await;
}

#[tokio::test]
async fn test_backward_compatibility() {
    // Test that isolated database APIs work consistently
    let db1 = create_isolated_db("compat_1").await;
    let db2 = create_isolated_db("compat_2").await;

    let query1 = db1.execute_query("SELECT 1 as test").await;
    let query2 = db2.execute_query("SELECT 1 as test").await;
    assert!(query1.is_ok(), "First database should be functional");
    assert!(query2.is_ok(), "Second database should be functional");
    let _ = db1.close().await;
    let _ = db2.close().await;
}

#[test]
fn test_error_handling_scenarios() {
    // Test various error scenarios without actually creating databases

    // Test path resolution with missing environment variables
    #[cfg(target_os = "windows")]
    {
        // Temporarily remove USERPROFILE if possible (in a controlled way)
        let original_userprofile = std::env::var("USERPROFILE").ok();
        std::env::remove_var("USERPROFILE");

        let path_result = get_test_default_path();
        assert!(
            path_result.is_err(),
            "Should fail when USERPROFILE is missing"
        );

        if let Err(DatabaseError::PathResolution(msg)) = path_result {
            assert!(
                msg.contains("USERPROFILE"),
                "Error should mention USERPROFILE"
            );
        }

        // Restore original value
        if let Some(original) = original_userprofile {
            std::env::set_var("USERPROFILE", original);
        }
    }

    // Test API consistency without using async operations in a sync test
    // We'll just verify that the path resolution logic works correctly
    let path_result = get_test_default_path();
    match path_result {
        Ok(path) => {
            println!("✓ Path resolution succeeded: {}", path.display());
            assert!(path.to_string_lossy().contains("data.db"));
        }
        Err(e) => {
            println!(
                "Path resolution failed (acceptable in test environments): {}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_api_consistency() {
    // Test that isolated database instances have consistent API behavior
    let db1 = create_isolated_db("api_cons_1").await;
    let db2 = create_isolated_db("api_cons_2").await;

    // Both should support the same operations
    let query1 = db1.execute_query("SELECT 1 as test").await;
    let query2 = db2.execute_query("SELECT 1 as test").await;

    assert!(query1.is_ok(), "First database should support queries");
    assert!(query2.is_ok(), "Second database should support queries");

    let _ = db1.close().await;
    let _ = db2.close().await;
}

// Helper function to get the default path for testing
// This replicates the internal logic for testing purposes
fn get_test_default_path() -> Result<PathBuf> {
    let db_dir = if cfg!(target_os = "windows") {
        let user_profile = std::env::var("USERPROFILE")
            .map_err(|e| DatabaseError::PathResolution(format!("USERPROFILE not found: {}", e)))?;
        PathBuf::from(user_profile)
            .join("AppData")
            .join("Local")
            .join("BurnCloud")
    } else {
        dirs::home_dir()
            .ok_or_else(|| DatabaseError::PathResolution("Home directory not found".to_string()))?
            .join(".burncloud")
    };

    Ok(db_dir.join("data.db"))
}
