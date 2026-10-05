use sqlx::{
    any::{AnyConnectOptions, AnyPoolOptions, AnyRow},
    AnyPool,
};
use std::str::FromStr;

use crate::error::{DatabaseError, Result};

/// Build a SQLite connection URL for a filesystem path.
///
/// **Extracted so the rules can be tested without touching the filesystem**, which the crate's plan asks for
/// ("先规划最小可测试接缝"). The logic lived inside `Database::new`, where exercising it meant creating a real
/// database and reading an environment variable.
///
/// The rules, and the defect the old form had:
///
/// * **An absolute path always needs three slashes.** `sqlite:///C:/data/db.sqlite` is the absolute form;
///   `sqlite://C:/data/db.sqlite` is parsed as a **host** named `C:` and fails with
///   `(code: 14) unable to open database file`. The same is true of a Unix-style path beginning with `/`, which
///   Windows accepts -- `sqlite:///tmp/db.sqlite` is absolute but `sqlite:///tmp...` written with two slashes is
///   not.
/// * **A relative path keeps two slashes**, because with three the leading `/` would become part of the path
///   and turn `data/db.sqlite` into `/data/db.sqlite`, i.e. an absolute path the caller did not ask for.
///
/// The previous condition was `is_windows() && !path.starts_with('/')`, which sent a Windows absolute path
/// written with forward slashes -- `/tmp/db.sqlite` -- down the **relative** branch, producing
/// `sqlite:///tmp/db.sqlite`'s broken sibling `sqlite://tmp/db.sqlite`. That is the exact failure this function
/// now prevents, and it is why the rule is about the path shape rather than the platform.
///
/// `is_windows` is still a parameter because the *separator* convention differs, and because a caller may be
/// building a URL for a database on another machine; it affects only how a bare drive path is recognised.
pub fn sqlite_url(path: &str, is_windows: bool, create_if_missing: bool) -> String {
    let suffix = if create_if_missing { "?mode=rwc" } else { "" };
    if is_absolute_path(path, is_windows) {
        format!("sqlite:///{}{}", path.trim_start_matches('/'), suffix)
    } else {
        format!("sqlite://{}{}", path, suffix)
    }
}

/// Whether `path` is absolute under the given platform's conventions.
///
/// Three shapes count: a leading `/` (POSIX, and accepted by Windows), a drive letter followed by `:` (Windows
/// only, so `C:foo` is not treated as absolute on Unix), and a UNC path (`//server/share`).
fn is_absolute_path(path: &str, is_windows: bool) -> bool {
    if path.starts_with('/') || path.starts_with("//") {
        return true;
    }
    if !is_windows {
        return false;
    }
    // `C:/...` or `C:\...` -- a drive letter, a colon, then a separator.
    let bytes: Vec<char> = path.chars().take(3).collect();
    bytes.len() == 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == ':'
        && (bytes[2] == '/' || bytes[2] == '\\')
}

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

            // BURNCLOUD_FRESH_DB=1: delete the SQLite file so the next connection
            // starts from a blank database. Used by CI/verification runners to
            // guarantee a clean state for first-user-is-admin tests.
            // Default is to preserve existing data. Set BURNCLOUD_FRESH_DB=1 to
            // explicitly request a fresh database (dev/CI scenario).
            let fresh_db = std::env::var("BURNCLOUD_FRESH_DB").as_deref() == Ok("1");
            if fresh_db && default_path.exists() {
                tracing::info!("BURNCLOUD_FRESH_DB: deleting {}", default_path.display());
                std::fs::remove_file(&default_path).map_err(|e| {
                    DatabaseError::DirectoryCreation(format!(
                        "failed to delete {}: {}",
                        default_path.display(),
                        e
                    ))
                })?;
            }

            let normalized_path = default_path
                .to_string_lossy()
                .to_string()
                .replace('\\', "/");
            // Ensure we use mode=rwc for SQLite to create file
            sqlite_url(&normalized_path, is_windows(), true)
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
        self.connection = Some(connection.clone());

        // Enable WAL mode for SQLite performance and concurrency
        if self.kind() == "sqlite" {
            let _ = sqlx::query("PRAGMA journal_mode=WAL;")
                .execute(connection.pool())
                .await;
        }

        // Run versioned DDL migrations first (creates all tables and columns).
        crate::migration::MigrationRunner::run(self).await?;

        // Run post-migration data fixups and seed initial records.
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

    /// 带参数的查询，返回单条记录或 None（防止 SQL 注入）
    pub async fn fetch_optional_with_params<T>(
        &self,
        query: &str,
        params: Vec<String>,
    ) -> Result<Option<T>>
    where
        T: for<'r> sqlx::FromRow<'r, AnyRow> + Send + Unpin,
    {
        let conn = self.get_connection()?;
        let mut query_builder = sqlx::query_as::<_, T>(query);

        for param in params {
            query_builder = query_builder.bind(param);
        }

        let result = query_builder.fetch_optional(conn.pool()).await?;
        Ok(result)
    }

    /// 带参数的查询，返回多条记录（防止 SQL 注入）
    pub async fn fetch_all_with_params<T>(&self, query: &str, params: Vec<String>) -> Result<Vec<T>>
    where
        T: for<'r> sqlx::FromRow<'r, AnyRow> + Send + Unpin,
    {
        let conn = self.get_connection()?;
        let mut query_builder = sqlx::query_as::<_, T>(query);

        for param in params {
            query_builder = query_builder.bind(param);
        }

        let results = query_builder.fetch_all(conn.pool()).await?;
        Ok(results)
    }
}

// Convenience function for creating a default database
pub async fn create_default_database() -> Result<Database> {
    Database::new().await
}

/// Create a database at the given URL (e.g. `"sqlite::memory:"` for testing).
pub async fn create_database_with_url(url: &str) -> Result<Database> {
    let mut db = Database {
        connection: None,
        database_url: url.to_string(),
    };
    db.initialize().await?;
    Ok(db)
}

// Platform detection and default path resolution functions
pub fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

pub fn get_default_database_path() -> Result<std::path::PathBuf> {
    let db_dir = if is_windows() {
        // Windows: %USERPROFILE%\AppData\Local\BurnCloud
        // USERPROFILE must be an absolute path. Empty/relative values would make
        // `AppData/...` relative to the current working directory; during tests that
        // previously created `crates/platform/storage/database/AppData/` inside the repo.
        let user_profile = std::env::var("USERPROFILE")
            .map_err(|e| DatabaseError::PathResolution(format!("USERPROFILE not found: {}", e)))?;
        let user_profile = user_profile.trim();
        if user_profile.is_empty() {
            return Err(DatabaseError::PathResolution(
                "USERPROFILE is empty".to_string(),
            ));
        }

        let user_profile = std::path::PathBuf::from(user_profile);
        if !user_profile.is_absolute() {
            return Err(DatabaseError::PathResolution(format!(
                "USERPROFILE must be an absolute path: {}",
                user_profile.display()
            )));
        }

        user_profile.join("AppData").join("Local").join("BurnCloud")
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
