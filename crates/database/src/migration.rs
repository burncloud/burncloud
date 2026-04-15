//! Versioned SQL migration runner.
//!
//! Migration files live under `crates/database/migrations/{sqlite,postgres}/`
//! and are embedded at compile time via `include_str!`.
//!
//! Each migration is run exactly once per database.  Applied versions are
//! tracked in the `_schema_migrations` table (created automatically on first
//! run).  All migration SQL is written to be idempotent so that existing
//! databases which pre-date the migration framework are upgraded safely.

use crate::{Database, DatabaseError, Result};
use std::time::{SystemTime, UNIX_EPOCH};

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// A single versioned migration (version identifier + SQL text).
struct Migration {
    version: &'static str,
    sql: &'static str,
}

// ---------------------------------------------------------------------------
// Migration catalogue — SQLite dialect
// ---------------------------------------------------------------------------

const MIGRATIONS_SQLITE: &[Migration] = &[
    Migration {
        version: "0001_initial_schema",
        sql: include_str!("../migrations/sqlite/0001_initial_schema.sql"),
    },
    Migration {
        version: "0002_alter_channels_add_columns",
        sql: include_str!("../migrations/sqlite/0002_alter_channels_add_columns.sql"),
    },
    Migration {
        version: "0003_alter_router_logs_add_columns",
        sql: include_str!("../migrations/sqlite/0003_alter_router_logs_add_columns.sql"),
    },
    Migration {
        version: "0004_alter_router_logs_add_cost_columns",
        sql: include_str!("../migrations/sqlite/0004_alter_router_logs_add_cost_columns.sql"),
    },
    Migration {
        version: "0005_alter_prices_add_multimodal",
        sql: include_str!("../migrations/sqlite/0005_alter_prices_add_multimodal.sql"),
    },
    Migration {
        version: "0006_alter_users_add_currency_balance",
        sql: include_str!("../migrations/sqlite/0006_alter_users_add_currency_balance.sql"),
    },
    Migration {
        version: "0007_alter_tiered_pricing_add_columns",
        sql: include_str!("../migrations/sqlite/0007_alter_tiered_pricing_add_columns.sql"),
    },
    Migration {
        version: "0008_alter_prices_add_extended",
        sql: include_str!("../migrations/sqlite/0008_alter_prices_add_extended.sql"),
    },
    Migration {
        version: "0009_router_tables",
        sql: include_str!("../migrations/sqlite/0009_router_tables.sql"),
    },
    Migration {
        version: "0010_rename_tables",
        sql: include_str!("../migrations/sqlite/0010_rename_tables.sql"),
    },
];

// ---------------------------------------------------------------------------
// Migration catalogue — PostgreSQL dialect
// ---------------------------------------------------------------------------

const MIGRATIONS_POSTGRES: &[Migration] = &[
    Migration {
        version: "0001_initial_schema",
        sql: include_str!("../migrations/postgres/0001_initial_schema.sql"),
    },
    Migration {
        version: "0002_alter_channels_add_columns",
        sql: include_str!("../migrations/postgres/0002_alter_channels_add_columns.sql"),
    },
    Migration {
        version: "0003_alter_router_logs_add_columns",
        sql: include_str!("../migrations/postgres/0003_alter_router_logs_add_columns.sql"),
    },
    Migration {
        version: "0004_alter_router_logs_add_cost_columns",
        sql: include_str!("../migrations/postgres/0004_alter_router_logs_add_cost_columns.sql"),
    },
    Migration {
        version: "0005_alter_prices_add_multimodal",
        sql: include_str!("../migrations/postgres/0005_alter_prices_add_multimodal.sql"),
    },
    Migration {
        version: "0006_alter_users_add_currency_balance",
        sql: include_str!("../migrations/postgres/0006_alter_users_add_currency_balance.sql"),
    },
    Migration {
        version: "0007_alter_tiered_pricing_add_columns",
        sql: include_str!("../migrations/postgres/0007_alter_tiered_pricing_add_columns.sql"),
    },
    Migration {
        version: "0008_alter_prices_add_extended",
        sql: include_str!("../migrations/postgres/0008_alter_prices_add_extended.sql"),
    },
    Migration {
        version: "0009_router_tables",
        sql: include_str!("../migrations/postgres/0009_router_tables.sql"),
    },
    Migration {
        version: "0010_rename_tables",
        sql: include_str!("../migrations/postgres/0010_rename_tables.sql"),
    },
];

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

pub struct MigrationRunner;

impl MigrationRunner {
    /// Apply all pending migrations for the connected database.
    ///
    /// # Idempotency
    /// - `CREATE TABLE IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS` — safe to
    ///   re-run.
    /// - `ALTER TABLE … ADD COLUMN` on SQLite does not support `IF NOT EXISTS`.
    ///   The runner swallows "duplicate column" and "already exists" errors so
    ///   that upgrading a database which already has the column is a no-op.
    /// - PostgreSQL uses `ADD COLUMN IF NOT EXISTS`, so no errors are expected.
    pub async fn run(db: &Database) -> Result<()> {
        let pool = db.get_connection()?.pool();
        let kind = db.kind();

        // Ensure the tracking table exists (idempotent, compatible with both
        // SQLite and PostgreSQL).
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS _schema_migrations (\
                version TEXT PRIMARY KEY, \
                applied_at INTEGER NOT NULL\
            )",
        )
        .execute(pool)
        .await?;

        let migrations: &[Migration] = if kind == "postgres" {
            MIGRATIONS_POSTGRES
        } else {
            MIGRATIONS_SQLITE
        };

        for migration in migrations {
            if Self::is_applied(pool, migration.version, &kind).await {
                continue;
            }

            Self::apply(pool, migration, &kind).await?;
        }

        Ok(())
    }

    /// Returns `true` when the given version is already recorded in the
    /// tracking table.
    async fn is_applied(pool: &sqlx::AnyPool, version: &str, kind: &str) -> bool {
        let count: i64 = if kind == "postgres" {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM _schema_migrations WHERE version = $1",
            )
            .bind(version)
            .fetch_one(pool)
            .await
            .unwrap_or(0)
        } else {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM _schema_migrations WHERE version = ?",
            )
            .bind(version)
            .fetch_one(pool)
            .await
            .unwrap_or(0)
        };
        count > 0
    }

    /// Execute every SQL statement in the migration file, then record the
    /// migration as applied.
    async fn apply(pool: &sqlx::AnyPool, migration: &Migration, kind: &str) -> Result<()> {
        for raw_stmt in migration.sql.split(';') {
            // Strip comments and whitespace; skip blank segments.
            let stmt: String = raw_stmt
                .lines()
                .filter(|l| !l.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n");
            let stmt = stmt.trim().to_string();
            if stmt.is_empty() {
                continue;
            }

            if let Err(e) = sqlx::query(&stmt).execute(pool).await {
                let msg = e.to_string().to_lowercase();
                // Idempotency: ignore errors that mean "already done".
                // These arise when ALTER TABLE is re-run on an existing column
                // (SQLite path) or when a table/index was already created.
                if msg.contains("duplicate column")
                    || msg.contains("already has a column named")
                    || msg.contains("already exists")
                {
                    // Expected: migration was partially or fully applied before
                    // the tracking table existed.
                } else {
                    return Err(DatabaseError::Migration(format!(
                        "migration '{}' failed: {}",
                        migration.version, e
                    )));
                }
            }
        }

        // Record the migration as successfully applied.
        let now = current_timestamp();
        if kind == "postgres" {
            sqlx::query(
                "INSERT INTO _schema_migrations (version, applied_at) VALUES ($1, $2)",
            )
            .bind(migration.version)
            .bind(now)
            .execute(pool)
            .await?;
        } else {
            sqlx::query(
                "INSERT INTO _schema_migrations (version, applied_at) VALUES (?, ?)",
            )
            .bind(migration.version)
            .bind(now)
            .execute(pool)
            .await?;
        }

        Ok(())
    }
}
