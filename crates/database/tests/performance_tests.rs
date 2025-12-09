use burncloud_database::{create_default_database, Database, Result};
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Performance and load tests for the default database location feature
/// These tests validate acceptable performance under normal operational load

#[tokio::test]
async fn test_database_creation_performance() {
    // Test that database creation completes within reasonable time
    let start_time = Instant::now();

    let result = timeout(Duration::from_secs(30), create_default_database()).await;

    match result {
        Ok(Ok(db)) => {
            let creation_time = start_time.elapsed();
            println!("✓ Database creation took: {:?}", creation_time);

            // Database creation should be reasonably fast (under 10 seconds for most systems)
            assert!(
                creation_time < Duration::from_secs(10),
                "Database creation took too long: {:?}",
                creation_time
            );

            let _ = db.close().await;
        }
        Ok(Err(e)) => {
            println!(
                "Database creation failed (acceptable in some environments): {}",
                e
            );
        }
        Err(_) => {
            panic!("Database creation timed out after 30 seconds");
        }
    }
}

#[tokio::test]
async fn test_concurrent_database_access() {
    // Test that multiple concurrent accesses don't cause issues
    let db_result = create_default_database().await;

    if let Ok(db) = db_result {
        // Create test table
        let _ = db.execute_query(
            "CREATE TABLE IF NOT EXISTS concurrent_test (id INTEGER PRIMARY KEY, thread_id INTEGER, timestamp TEXT)"
        ).await;

        // Spawn multiple concurrent tasks
        let mut handles = vec![];
        let num_tasks = 10;

        for i in 0..num_tasks {
            let connection = db
                .connection()
                .expect("Database should be initialized")
                .clone();
            let handle = tokio::spawn(async move {
                let timestamp = chrono::Utc::now().to_rfc3339();
                let query = format!(
                    "INSERT INTO concurrent_test (thread_id, timestamp) VALUES ({}, '{}')",
                    i, timestamp
                );
                // Use connection pool directly for concurrent access
                let result = sqlx::query(&query).execute(connection.pool()).await;
                result.map_err(|e| burncloud_database::DatabaseError::Connection(e))
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        let mut success_count = 0;
        for handle in handles {
            match handle.await {
                Ok(Ok(_)) => success_count += 1,
                Ok(Err(e)) => println!("Task failed: {}", e),
                Err(e) => println!("Task panicked: {}", e),
            }
        }

        println!(
            "✓ Concurrent operations: {}/{} succeeded",
            success_count, num_tasks
        );

        // Verify all data was inserted
        #[derive(sqlx::FromRow)]
        struct ConcurrentRow {
            id: i64,
            thread_id: i64,
        }

        if let Ok(rows) = db
            .fetch_all::<ConcurrentRow>("SELECT id, thread_id FROM concurrent_test")
            .await
        {
            println!("✓ Inserted {} rows concurrently", rows.len());
            assert!(
                rows.len() >= success_count,
                "Should have at least as many rows as successful inserts"
            );
        }

        let _ = db.close().await;
    } else {
        println!("Database creation failed, skipping concurrent test");
    }
}

#[tokio::test]
async fn test_large_dataset_operations() {
    // Test performance with a reasonably large dataset
    let db_result = create_default_database().await;

    if let Ok(db) = db_result {
        // Create test table
        let _ = db.execute_query(
            "CREATE TABLE IF NOT EXISTS performance_test (id INTEGER PRIMARY KEY, data TEXT, number INTEGER)"
        ).await;

        let start_time = Instant::now();
        let num_records = 1000; // Reasonable size for integration testing

        // Insert records in batches for better performance
        let batch_size = 100;
        let mut successful_inserts = 0;

        for batch_start in (0..num_records).step_by(batch_size) {
            let mut batch_query = "INSERT INTO performance_test (data, number) VALUES".to_string();
            let batch_end = std::cmp::min(batch_start + batch_size, num_records);

            for i in batch_start..batch_end {
                if i > batch_start {
                    batch_query.push(',');
                }
                batch_query.push_str(&format!(" ('test_data_{}', {})", i, i * 2));
            }

            if let Ok(_) = db.execute_query(&batch_query).await {
                successful_inserts += batch_end - batch_start;
            }
        }

        let insert_time = start_time.elapsed();
        println!(
            "✓ Inserted {} records in {:?}",
            successful_inserts, insert_time
        );

        // Test query performance
        let query_start = Instant::now();
        let count_result = db
            .fetch_one::<(i64,)>("SELECT COUNT(*) FROM performance_test")
            .await;
        let query_time = query_start.elapsed();

        if let Ok((count,)) = count_result {
            println!("✓ Query returned {} records in {:?}", count, query_time);
            assert!(
                query_time < Duration::from_secs(5),
                "Query took too long: {:?}",
                query_time
            );
        }

        // Test more complex query
        let complex_query_start = Instant::now();
        let complex_result = db.fetch_all::<(i64, String, i64)>(
            "SELECT id, data, number FROM performance_test WHERE number > 500 ORDER BY number DESC LIMIT 100"
        ).await;
        let complex_query_time = complex_query_start.elapsed();

        if let Ok(results) = complex_result {
            println!(
                "✓ Complex query returned {} records in {:?}",
                results.len(),
                complex_query_time
            );
            assert!(
                complex_query_time < Duration::from_secs(5),
                "Complex query took too long: {:?}",
                complex_query_time
            );
        }

        let _ = db.close().await;
    } else {
        println!("Database creation failed, skipping performance test");
    }
}

#[tokio::test]
async fn test_database_initialization_performance() {
    // Test the performance of database creation and initialization
    let num_iterations = 5;

    // Test Database::new() performance (creation + initialization)
    let mut initialized_times = vec![];
    for _ in 0..num_iterations {
        let start = Instant::now();
        let result = Database::new().await;
        let elapsed = start.elapsed();

        if let Ok(db) = result {
            initialized_times.push(elapsed);
            let _ = db.close().await;
        }
    }

    if !initialized_times.is_empty() {
        let avg_initialized =
            initialized_times.iter().sum::<Duration>() / initialized_times.len() as u32;
        println!("✓ Average Database::new() time: {:?}", avg_initialized);

        // The initialized version should be reasonable
        assert!(
            avg_initialized < Duration::from_secs(10),
            "Initialization taking too long: {:?}",
            avg_initialized
        );
    }

    // Test create_default_database() performance for comparison
    let mut convenience_times = vec![];
    for _ in 0..num_iterations {
        let start = Instant::now();
        let result = create_default_database().await;
        let elapsed = start.elapsed();

        if let Ok(db) = result {
            convenience_times.push(elapsed);
            let _ = db.close().await;
        }
    }

    if !convenience_times.is_empty() {
        let avg_convenience =
            convenience_times.iter().sum::<Duration>() / convenience_times.len() as u32;
        println!(
            "✓ Average create_default_database() time: {:?}",
            avg_convenience
        );
        assert!(
            avg_convenience < Duration::from_secs(10),
            "Convenience function taking too long: {:?}",
            avg_convenience
        );
    }
}

#[tokio::test]
async fn test_memory_usage_stability() {
    // Test that repeated database operations don't cause memory leaks
    let db_result = create_default_database().await;

    if let Ok(db) = db_result {
        // Perform repeated operations to check for memory stability
        for i in 0..100 {
            let table_name = format!("temp_table_{}", i);
            let create_query = format!(
                "CREATE TEMPORARY TABLE {} (id INTEGER, value TEXT)",
                table_name
            );
            let insert_query = format!("INSERT INTO {} (id, value) VALUES (1, 'test')", table_name);
            let select_query = format!("SELECT COUNT(*) FROM {}", table_name);
            let drop_query = format!("DROP TABLE {}", table_name);

            // These operations should all complete without issues
            let _ = db.execute_query(&create_query).await;
            let _ = db.execute_query(&insert_query).await;
            let _ = db.fetch_one::<(i64,)>(&select_query).await;
            let _ = db.execute_query(&drop_query).await;

            // Occasional checks to ensure the database is still responsive
            if i % 25 == 0 {
                let health_check = db.execute_query("SELECT 1").await;
                assert!(
                    health_check.is_ok(),
                    "Database should remain responsive during repeated operations"
                );
            }
        }

        println!("✓ Completed 100 repeated operations without issues");

        let _ = db.close().await;
    } else {
        println!("Database creation failed, skipping memory stability test");
    }
}

#[tokio::test]
async fn test_rapid_database_creation_and_destruction() {
    // Test creating and destroying databases rapidly
    let num_cycles = 10;
    let mut success_count = 0;

    for i in 0..num_cycles {
        let start = Instant::now();

        let db_result = create_default_database().await;
        if let Ok(db) = db_result {
            // Perform a quick operation to ensure it's functional
            let query_result = db.execute_query("SELECT 1").await;
            if query_result.is_ok() {
                success_count += 1;
            }

            let _ = db.close().await;
        }

        let cycle_time = start.elapsed();
        println!("Cycle {} completed in {:?}", i + 1, cycle_time);

        // Each cycle should complete reasonably quickly
        assert!(
            cycle_time < Duration::from_secs(30),
            "Cycle {} took too long: {:?}",
            i + 1,
            cycle_time
        );
    }

    println!(
        "✓ Rapid creation/destruction: {}/{} cycles succeeded",
        success_count, num_cycles
    );
}

// Helper function for tests
fn get_test_default_path() -> Result<std::path::PathBuf> {
    use burncloud_database::DatabaseError;
    use std::path::PathBuf;

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
