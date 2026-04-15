//! Post-migration data fixups and seed data initialisation.
//!
//! The DDL (CREATE TABLE / ALTER TABLE) that used to live here has been moved
//! to versioned `.sql` files under `crates/database/migrations/` and is now
//! executed by [`crate::migration::MigrationRunner`] before this module runs.
//!
//! This module is responsible for:
//! 1. **Complex data migrations** — one-time data transformations that are too
//!    dynamic for plain SQL files (e.g. conditional type coercions, table
//!    renames with data copy).
//! 2. **Seed data** — inserting the demo user/token and default protocol
//!    configs when they don't yet exist.
//!
//! The implementation is split into domain sub-modules:
//! - [`rename`] — full table rename data migrations (legacy → canonical names)
//! - [`router`] — router_logs table fixups
//! - [`price`]  — price table migrations and format conversions
//! - [`user`]   — token schema migration, quota conversion and seed data

mod price;
mod rename;
mod router;
mod user;

use crate::{Database, Result};

/// Get current Unix timestamp in seconds.
/// Returns 0 if system time is before Unix epoch (extremely unlikely).
pub(crate) fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub struct Schema;

impl Schema {
    /// Run data migrations and seed initial records.
    ///
    /// This is called **after** [`crate::migration::MigrationRunner::run`] has
    /// applied all DDL migrations, so all tables and columns are guaranteed to
    /// exist when this function runs.
    pub async fn init(db: &Database) -> Result<()> {
        let pool = db.get_connection()?.pool();
        let kind = db.kind();

        // Run table renames first so that subsequent migrations and seed data
        // address the canonical table names.
        rename::migrate_table_renames(pool, &kind).await?;

        router::migrate_router_logs(pool, &kind).await?;
        price::migrate_prices(pool, &kind).await?;
        user::migrate_users_and_seed(pool, &kind).await?;

        Ok(())
    }
}
