use burncloud_database::{Result, Database, create_default_database};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== BurnCloud Database Core - Default Location Example ===\n");

    // Method 1: Using the convenience function
    println!("1. Creating database with default location using create_default_database()...");
    match create_default_database().await {
        Ok(db) => {
            println!("✓ Default database created successfully!");

            // Perform some basic operations
            let result = db.execute_query("CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT)").await?;
            println!("✓ Settings table created: {:?}", result);

            let insert_result = db.execute_query("INSERT OR REPLACE INTO settings (key, value) VALUES ('app_version', '1.0.0')").await?;
            println!("✓ Setting inserted: {:?}", insert_result);

            db.close().await?;
            println!("✓ Database closed successfully\n");
        }
        Err(e) => {
            println!("⚠ Could not create default database (this may be due to environment configuration): {}", e);
            println!("This is normal in some testing environments.\n");
        }
    }

    // Method 2: Using the new simplified constructor
    println!("2. Creating database with Database::new()...");
    match Database::new().await {
        Ok(db) => {
            println!("✓ Database created and initialized in one step!");

            // Perform a quick test
            let result = db.execute_query("CREATE TABLE IF NOT EXISTS test_table (id INTEGER PRIMARY KEY)").await?;
            println!("✓ Test table created: {:?}", result);

            db.close().await?;
            println!("✓ Database closed successfully");
        }
        Err(e) => {
            println!("⚠ Could not create database: {}", e);
        }
    }

    // Show the default path that would be used
    println!("\n3. Default database location:");
    println!("Platform: {}", if cfg!(target_os = "windows") { "Windows" } else { "Linux/Unix" });

    // This uses internal function logic to show the path
    let expected_path = if cfg!(target_os = "windows") {
        std::env::var("USERPROFILE").map(|profile|
            format!("{}\\AppData\\Local\\BurnCloud\\data.db", profile)
        ).unwrap_or_else(|_| "Could not determine USERPROFILE".to_string())
    } else {
        dirs::home_dir().map(|home|
            format!("{}/.burncloud/data.db", home.display())
        ).unwrap_or_else(|| "Could not determine home directory".to_string())
    };

    println!("Default path: {}", expected_path);

    println!("\n=== Example completed ===");
    Ok(())
}