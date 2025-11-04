use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

use crate::error::{DatabaseError, Result};

#[derive(Clone)]
pub struct DatabaseConnection {
    pool: SqlitePool,
}

impl DatabaseConnection {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn close(self) {
        self.pool.close().await;
    }
}

pub struct Database {
    connection: Option<DatabaseConnection>,
    database_path: String,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let default_path = get_default_database_path()?;

        create_directory_if_not_exists(&default_path)?;

        let path = default_path.to_string_lossy().to_string();
        let mut db = Self {
            connection: None,
            database_path: path,
        };
        db.initialize().await?;
        Ok(db)
    }

    pub async fn initialize(&mut self) -> Result<()> {
        let database_url = if self.database_path == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            // Normalize path separators for SQLite URL
            // SQLite requires forward slashes even on Windows
            let normalized_path = self.database_path.replace('\\', "/");
            // Add mode=rwc to create the database file if it doesn't exist
            format!("sqlite://{}?mode=rwc", normalized_path)
        };

        let connection = DatabaseConnection::new(&database_url).await?;

        self.connection = Some(connection);
        Ok(())
    }

    pub fn connection(&self) -> Result<&DatabaseConnection> {
        self.connection
            .as_ref()
            .ok_or(DatabaseError::NotInitialized)
    }

    pub async fn create_tables(&self) -> Result<()> {
        let _conn = self.connection()?;

        Ok(())
    }

    pub async fn close(mut self) -> Result<()> {
        if let Some(connection) = self.connection.take() {
            connection.close().await;
        }
        Ok(())
    }

    pub async fn execute_query(&self, query: &str) -> Result<sqlx::sqlite::SqliteQueryResult> {
        let conn = self.connection()?;
        let result = sqlx::query(query).execute(conn.pool()).await?;
        Ok(result)
    }

    pub async fn execute_query_with_params(&self, query: &str, params: Vec<String>) -> Result<sqlx::sqlite::SqliteQueryResult> {
        let conn = self.connection()?;
        let mut query_builder = sqlx::query(query);

        for param in params {
            query_builder = query_builder.bind(param);
        }

        let result = query_builder.execute(conn.pool()).await?;
        Ok(result)
    }

    pub async fn query(&self, query: &str) -> Result<Vec<sqlx::sqlite::SqliteRow>> {
        let conn = self.connection()?;
        let rows = sqlx::query(query).fetch_all(conn.pool()).await?;
        Ok(rows)
    }

    pub async fn query_with_params(&self, query: &str, params: Vec<String>) -> Result<Vec<sqlx::sqlite::SqliteRow>> {
        let conn = self.connection()?;
        let mut query_builder = sqlx::query(query);

        for param in params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder.fetch_all(conn.pool()).await?;
        Ok(rows)
    }

    pub async fn fetch_one<T>(&self, query: &str) -> Result<T>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> + Send + Unpin,
    {
        let conn = self.connection()?;
        let result = sqlx::query_as::<_, T>(query).fetch_one(conn.pool()).await?;
        Ok(result)
    }

    pub async fn fetch_all<T>(&self, query: &str) -> Result<Vec<T>>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> + Send + Unpin,
    {
        let conn = self.connection()?;
        let results = sqlx::query_as::<_, T>(query).fetch_all(conn.pool()).await?;
        Ok(results)
    }

    pub async fn fetch_optional<T>(&self, query: &str) -> Result<Option<T>>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> + Send + Unpin,
    {
        let conn = self.connection()?;
        let result = sqlx::query_as::<_, T>(query).fetch_optional(conn.pool()).await?;
        Ok(result)
    }
}

// Convenience function for creating a default database
pub async fn create_default_database() -> Result<Database> {
    Database::new().await
}

// Platform detection and default path resolution functions
pub fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

pub fn get_default_database_path() -> Result<std::path::PathBuf> {
    let db_dir = if is_windows() {
        // Windows: %USERPROFILE%\AppData\Local\BurnCloud
        let user_profile = std::env::var("USERPROFILE")
            .map_err(|e| DatabaseError::PathResolution(format!("USERPROFILE not found: {}", e)))?;
        std::path::PathBuf::from(user_profile)
            .join("AppData")
            .join("Local")
            .join("BurnCloud")
    } else {
        // Linux: ~/.burncloud
        dirs::home_dir()
            .ok_or_else(|| DatabaseError::PathResolution("Home directory not found".to_string()))?
            .join(".burncloud")
    };

    Ok(db_dir.join("data.db"))
}

fn create_directory_if_not_exists(path: &std::path::Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DatabaseError::DirectoryCreation(format!("{}: {}", parent.display(), e)))?;
        }
    }
    Ok(())
}

