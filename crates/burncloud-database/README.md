# BurnCloud Database

Core database abstractions and SQLite implementation for BurnCloud AI management system.

## Features

- SQLite database support using sqlx
- Async/await API
- Connection pooling
- Error handling with detailed error types
- Both file-based and in-memory database support

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
burncloud-database = "0.1.0"
```

### Basic Usage

```rust
use burncloud_database::{create_database, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Create a database
    let mut db = create_database("./my_database.db").await?;

    // Create a table
    db.execute_query(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE
        )"
    ).await?;

    // Insert data
    db.execute_query(
        "INSERT INTO users (name, email) VALUES ('John Doe', 'john@example.com')"
    ).await?;

    // Query data
    let users: Vec<(i64, String, String)> = db.fetch_all(
        "SELECT id, name, email FROM users"
    ).await?;

    for (id, name, email) in users {
        println!("User {}: {} ({})", id, name, email);
    }

    // Close the database
    db.close().await?;
    Ok(())
}
```

### In-Memory Database

```rust
use burncloud_database::{create_in_memory_database, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut db = create_in_memory_database().await?;

    // Use the database same as file-based database
    // The database will be destroyed when the connection is closed

    db.close().await?;
    Ok(())
}
```

### Direct Database Control

```rust
use burncloud_database::{Database, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut db = Database::new("./my_database.db");
    db.initialize().await?;

    // Use the database...

    db.close().await?;
    Ok(())
}
```

## API Reference

### Database

The main database struct that provides all database operations.

#### Methods

- `new(path)` - Create a new database instance with file path
- `new_in_memory()` - Create a new in-memory database instance
- `initialize()` - Initialize the database connection
- `connection()` - Get the database connection
- `execute_query(query)` - Execute a SQL query
- `fetch_one<T>(query)` - Fetch a single row
- `fetch_all<T>(query)` - Fetch all rows
- `fetch_optional<T>(query)` - Fetch optional row
- `close()` - Close the database connection

### Convenience Functions

- `create_database(path)` - Create and initialize a file-based database
- `create_in_memory_database()` - Create and initialize an in-memory database

## Error Handling

The library provides comprehensive error handling through the `DatabaseError` enum:

- `Connection` - Database connection errors
- `Migration` - Database migration errors
- `Query` - SQL query errors
- `Serialization` - JSON serialization errors
- `NotInitialized` - Database not initialized
- `Io` - IO errors

## Examples

Run the basic usage example:

```bash
cargo run --example basic_usage
```

## License

This project is licensed under either of

- Apache License, Version 2.0
- MIT License

at your option.