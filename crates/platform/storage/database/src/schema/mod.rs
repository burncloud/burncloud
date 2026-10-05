//! Post-migration data fixups and seed data initialisation.
//!
//! The DDL (CREATE TABLE / ALTER TABLE) that used to live here has been moved
//! to versioned `.sql` files under `crates/platform/storage/database/migrations/` and is now
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

#[expect(
    clippy::cognitive_complexity,
    reason = "price migrations are legacy one-shot schema/data compatibility code; #711 made every fallible SQL step explicit, while decomposition is tracked separately from correctness"
)]
mod price;
#[expect(
    clippy::cognitive_complexity,
    reason = "legacy rename migration intentionally branches across old table shapes; #711 fixed failure propagation and this expectation keeps only the remaining maintainability debt visible"
)]
mod rename;
#[expect(
    clippy::cognitive_complexity,
    reason = "router-log schema migration handles multiple historical layouts; SQL failures now propagate after #711, leaving only decomposition debt"
)]
mod router;
#[expect(
    clippy::cognitive_complexity,
    reason = "user/token migration preserves several historical schema variants; SQL failures now propagate after #711, leaving only decomposition debt"
)]
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
