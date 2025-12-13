use sqlx::{
    any::{AnyConnectOptions, AnyPoolOptions, AnyRow},
    AnyPool,
};
use std::str::FromStr;

use crate::error::{DatabaseError, Result};

#[derive(Clone)]
pub struct DatabaseConnection {
    pool: AnyPool,
}

impl DatabaseConnection {
    pub async fn new(database_url: &str) -> Result<Self> {
        // Handle SQLite specific options via URL string modification if needed.
        // For AnyPool, parsing the URL usually sets up the correct options.
        // We rely on the caller to provide a correct URL (e.g. with ?mode=rwc).

        let options =
            AnyConnectOptions::from_str(database_url).map_err(DatabaseError::Connection)?;

        let pool = AnyPoolOptions::new()
            .max_connections(10)
            .connect_with(options)
            .await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &AnyPool {
        &self.pool
    }

    pub async fn close(self) {
        self.pool.close().await;
    }
}

pub struct Database {
    connection: Option<DatabaseConnection>,
    database_url: String,
}

impl Database {
    pub async fn new() -> Result<Self> {
        // Check environment variable for DB connection
        let database_url = if let Ok(url) = std::env::var("BURNCLOUD_DATABASE_URL") {
            url
        } else {
            // Default to local SQLite
            let default_path = get_default_database_path()?;
            create_directory_if_not_exists(&default_path)?;
            let normalized_path = default_path
                .to_string_lossy()
                .to_string()
                .replace('\\', "/");
            // Ensure we use mode=rwc for SQLite to create file
            if is_windows() && !normalized_path.starts_with('/') {
                format!("sqlite:///{}?mode=rwc", normalized_path)
            } else {
                format!("sqlite://{}?mode=rwc", normalized_path)
            }
        };

        let mut db = Self {
            connection: None,
            database_url,
        };
        db.initialize().await?;
        Ok(db)
    }

    pub async fn initialize(&mut self) -> Result<()> {
        sqlx::any::install_default_drivers();
        let connection = DatabaseConnection::new(&self.database_url).await?;
        self.connection = Some(connection);

        // Enable WAL mode for SQLite performance and concurrency
        if self.kind() == "sqlite" {
            let _ = sqlx::query("PRAGMA journal_mode=WAL;")
                .execute(self.connection.as_ref().unwrap().pool())
                .await;
        }

        // Initialize New API Schema
        crate::schema::Schema::init(self).await?;

        Ok(())
    }

    pub fn get_connection(&self) -> Result<&DatabaseConnection> {
        self.connection
            .as_ref()
            .ok_or(DatabaseError::NotInitialized)
    }

    pub fn kind(&self) -> String {
        if self.database_url.starts_with("postgres") {
            "postgres".to_string()
        } else {
            "sqlite".to_string()
        }
    }

    pub async fn create_tables(&self) -> Result<()> {
        let _conn = self.get_connection()?;
        Ok(())
    }

    pub async fn close(mut self) -> Result<()> {
        if let Some(connection) = self.connection.take() {
            connection.close().await;
        }
        Ok(())
    }

    pub async fn execute_query(&self, query: &str) -> Result<sqlx::any::AnyQueryResult> {
        let conn = self.get_connection()?;
        let result = sqlx::query(query).execute(conn.pool()).await?;
        Ok(result)
    }

    pub async fn execute_query_with_params(
        &self,
        query: &str,
        params: Vec<String>,
    ) -> Result<sqlx::any::AnyQueryResult> {
        let conn = self.get_connection()?;
        let mut query_builder = sqlx::query(query);

        for param in params {
            query_builder = query_builder.bind(param);
        }

        let result = query_builder.execute(conn.pool()).await?;
        Ok(result)
    }

    pub async fn query(&self, query: &str) -> Result<Vec<AnyRow>> {
        let conn = self.get_connection()?;
        let rows = sqlx::query(query).fetch_all(conn.pool()).await?;
        Ok(rows)
    }

    pub async fn query_with_params(&self, query: &str, params: Vec<String>) -> Result<Vec<AnyRow>> {
        let conn = self.get_connection()?;
        let mut query_builder = sqlx::query(query);

        for param in params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder.fetch_all(conn.pool()).await?;
        Ok(rows)
    }

    pub async fn fetch_one<T>(&self, query: &str) -> Result<T>
    where
        T: for<'r> sqlx::FromRow<'r, AnyRow> + Send + Unpin,
    {
        let conn = self.get_connection()?;
        let result = sqlx::query_as::<_, T>(query).fetch_one(conn.pool()).await?;
        Ok(result)
    }

    pub async fn fetch_all<T>(&self, query: &str) -> Result<Vec<T>>
    where
        T: for<'r> sqlx::FromRow<'r, AnyRow> + Send + Unpin,
    {
        let conn = self.get_connection()?;
        let results = sqlx::query_as::<_, T>(query).fetch_all(conn.pool()).await?;
        Ok(results)
    }

    pub async fn fetch_optional<T>(&self, query: &str) -> Result<Option<T>>
    where
        T: for<'r> sqlx::FromRow<'r, AnyRow> + Send + Unpin,
    {
        let conn = self.get_connection()?;
        let result = sqlx::query_as::<_, T>(query)
            .fetch_optional(conn.pool())
            .await?;
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
            std::fs::create_dir_all(parent).map_err(|e| {
                DatabaseError::DirectoryCreation(format!("{}: {}", parent.display(), e))
            })?;
        }
    }
    Ok(())
}
