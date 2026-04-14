pub mod database;
pub mod error;
pub mod placeholder;
pub mod schema;

pub use database::{
    create_database_with_url, create_default_database, get_default_database_path, is_windows,
    Database, DatabaseConnection,
};
pub use error::{DatabaseError, Result};
pub use placeholder::{adapt_sql, ph, phs};

pub use sqlx;
