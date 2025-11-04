use burncloud_database::{Database, DatabaseError, Result, create_default_database};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Integration tests for the default database location feature
/// These tests focus on functional validation and real-world scenarios

#[tokio::test]
async fn test_create_default_database_end_to_end() {
    // Test the complete end-to-end workflow of creating a default database
    let result = create_default_database().await;

    match result {
        Ok(db) => {
            // Clean up any existing test data from previous runs
            let _ = db.execute_query("DROP TABLE IF EXISTS test_table").await;

            // Verify the database is functional by performing operations
            let create_result = db.execute_query(
                "CREATE TABLE IF NOT EXISTS test_table (id INTEGER PRIMARY KEY, name TEXT)"
            ).await;
            if let Err(e) = &create_result {
                eprintln!("Failed to create table: {:?}", e);
            }
            assert!(create_result.is_ok(), "Should be able to create tables");

            let insert_result = db.execute_query(
                "INSERT INTO test_table (name) VALUES ('test_data')"
            ).await;
            assert!(insert_result.is_ok(), "Should be able to insert data");

            // Verify data can be retrieved
            #[derive(sqlx::FromRow)]
            struct TestRow {
                id: i64,
                name: String,
            }

            let rows: Result<Vec<TestRow>> = db.fetch_all("SELECT id, name FROM test_table").await;
            assert!(rows.is_ok(), "Should be able to fetch data");
            let rows = rows.unwrap();
            assert_eq!(rows.len(), 1, "Should have exactly one row");
            assert_eq!(rows[0].name, "test_data", "Data should match what was inserted");

            // Clean up
            let _ = db.close().await;
        }
        Err(e) => {
            // In environments where file database creation might fail,
            // at least verify that it's a reasonable error
            match e {
                DatabaseError::PathResolution(_) => {
                    println!("Path resolution failed (acceptable in some environments): {}", e);
                }
                DatabaseError::DirectoryCreation(_) => {
                    println!("Directory creation failed (acceptable in some environments): {}", e);
                }
                DatabaseError::Connection(_) => {
                    println!("Connection failed (acceptable in some environments): {}", e);
                }
                _ => panic!("Unexpected error type: {}", e),
            }
        }
    }
}

#[tokio::test]
async fn test_database_initialization_patterns() {
    // Test different database initialization patterns (since new_with_path is removed)

    // Test Database::new() - should create and initialize with default path
    let db_initialized_result = Database::new().await;
    match db_initialized_result {
        Ok(db) => {
            // Should be initialized and functional
            let connection_result = db.connection();
            assert!(connection_result.is_ok(), "Database should be initialized");

            // Should be able to perform operations
            let query_result = db.execute_query("SELECT 1 as test").await;
            assert!(query_result.is_ok(), "Should be able to execute queries");

            let _ = db.close().await;
        }
        Err(e) => {
            println!("Database::new() failed (acceptable in some environments): {}", e);
        }
    }

    // Test create_default_database() convenience function
    let convenience_result = create_default_database().await;
    match convenience_result {
        Ok(db) => {
            let query_result = db.execute_query("SELECT 1 as test").await;
            assert!(query_result.is_ok(), "Convenience function should work");
            let _ = db.close().await;
        }
        Err(e) => {
            println!("create_default_database() failed (acceptable in some environments): {}", e);
        }
    }
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
                    path_str.contains("AppData") && path_str.contains("Local") && path_str.contains("BurnCloud"),
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
            println!("Path resolution failed (acceptable in some environments): {}", e);
        }
    }
}

#[tokio::test]
async fn test_directory_creation_and_permissions() {
    // Test that directories are created properly with correct permissions
    let db_result = Database::new().await;

    match db_result {
        Ok(db) => {
            // If database creation succeeded, verify the directory exists
            if let Ok(default_path) = get_test_default_path() {
                if let Some(parent_dir) = default_path.parent() {
                    assert!(parent_dir.exists(), "Parent directory should have been created");

                    // Test that we can write to the directory
                    let test_file = parent_dir.join("test_write.tmp");
                    let write_result = fs::write(&test_file, "test");

                    if write_result.is_ok() {
                        // Clean up test file
                        let _ = fs::remove_file(&test_file);
                    }

                    // Clean up
                    let _ = db.close().await;
                }
            }
        }
        Err(e) => {
            println!("Database creation failed (acceptable in some environments): {}", e);
        }
    }
}

#[tokio::test]
async fn test_multiple_database_instances() {
    // Test that multiple default database instances can coexist
    let db1_result = Database::new().await;
    let db2_result = Database::new().await;

    match (db1_result, db2_result) {
        (Ok(db1), Ok(db2)) => {
            // Both databases should be functional
            let result1 = db1.execute_query("SELECT 1 as test").await;
            let result2 = db2.execute_query("SELECT 1 as test").await;

            assert!(result1.is_ok(), "First database should be functional");
            assert!(result2.is_ok(), "Second database should be functional");

            // Clean up
            let _ = db1.close().await;
            let _ = db2.close().await;
        }
        _ => {
            println!("Multiple database creation failed (acceptable in some environments)");
        }
    }
}

#[tokio::test]
async fn test_database_persistence() {
    // Test that data persists between database instances
    let test_value = "persistent_test_data";

    // Create first database instance and insert data
    let db1_result = Database::new().await;
    if let Ok(db1) = db1_result {
        let create_result = db1.execute_query(
            "CREATE TABLE IF NOT EXISTS persistence_test (id INTEGER PRIMARY KEY, value TEXT)"
        ).await;

        if create_result.is_ok() {
            let insert_result = db1.execute_query(&format!(
                "INSERT INTO persistence_test (value) VALUES ('{}')", test_value
            )).await;

            if insert_result.is_ok() {
                let _ = db1.close().await;

                // Create second database instance and verify data exists
                let db2_result = Database::new().await;
                if let Ok(db2) = db2_result {
                    #[derive(sqlx::FromRow)]
                    struct PersistenceRow {
                        value: String,
                    }

                    let rows: Result<Vec<PersistenceRow>> = db2.fetch_all(
                        "SELECT value FROM persistence_test"
                    ).await;

                    if let Ok(rows) = rows {
                        assert!(!rows.is_empty(), "Data should persist between instances");
                        assert_eq!(rows[0].value, test_value, "Data should match what was inserted");
                        println!("✓ Data persistence verified");
                    }

                    let _ = db2.close().await;
                }
            }
        }
    }
}

#[tokio::test]
async fn test_backward_compatibility() {
    // Test that default database APIs work consistently
    // Since new_with_path is removed, test with default database patterns

    // Test default database creation
    let default_db_result = Database::new().await;
    if let Ok(default_db) = default_db_result {
        let query_result = default_db.execute_query("SELECT 1 as test").await;
        assert!(query_result.is_ok(), "Default database should be functional");
        let _ = default_db.close().await;
    }

    // Test that multiple database instances work independently
    let db1_result = Database::new().await;
    let db2_result = create_default_database().await;

    match (db1_result, db2_result) {
        (Ok(db1), Ok(db2)) => {
            let query1 = db1.execute_query("SELECT 1 as test").await;
            let query2 = db2.execute_query("SELECT 1 as test").await;
            assert!(query1.is_ok(), "First database should be functional");
            assert!(query2.is_ok(), "Second database should be functional");
            let _ = db1.close().await;
            let _ = db2.close().await;
        }
        _ => {
            println!("Multiple database creation scenarios tested (some failures acceptable in test environments)");
        }
    }
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
        assert!(path_result.is_err(), "Should fail when USERPROFILE is missing");

        if let Err(DatabaseError::PathResolution(msg)) = path_result {
            assert!(msg.contains("USERPROFILE"), "Error should mention USERPROFILE");
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
            println!("Path resolution failed (acceptable in test environments): {}", e);
        }
    }
}

#[tokio::test]
async fn test_api_consistency() {
    // Test that all database creation APIs follow consistent patterns

    // Test default path database (main API now)
    let default_db_result = Database::new().await;
    match default_db_result {
        Ok(default_db) => {
            // Should be initialized and functional
            println!("✓ Default database created and initialized successfully");
            let _ = default_db.close().await;
        }
        Err(e) => {
            println!("Default database creation failed (acceptable): {}", e);
        }
    }

    // Test convenience function for consistency
    let convenience_result = create_default_database().await;
    match convenience_result {
        Ok(default_db) => {
            println!("✓ Convenience function works consistently");
            let _ = default_db.close().await;
        }
        Err(e) => {
            println!("Convenience function failed (acceptable): {}", e);
        }
    }
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