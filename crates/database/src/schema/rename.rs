//! Table rename data migrations.
//!
//! Copies rows from legacy table names into the new canonical names created by
//! migration `0010_rename_tables`, then drops the old tables.
//!
//! ## Rename order (conflict-safe)
//!
//! ```text
//! Step 1:  user_roles          → user_role_bindings   (frees name for step 2)
//! Step 2:  roles               → user_roles
//! Step 3:  users               → user_accounts
//! Step 4:  recharges           → user_recharges        (child FK to users dropped)
//! Step 5:  channels            → channel_providers
//! Step 6:  abilities           → channel_abilities
//! Step 7:  tokens              → user_api_keys
//! Step 8:  prices              → billing_prices
//! Step 9:  protocol_configs    → channel_protocol_configs
//! Step 10: tiered_pricing      → billing_tiered_prices
//! Step 11: exchange_rates      → billing_exchange_rates
//! Step 12: video_tasks         → router_video_tasks
//! Step 13: setting             → sys_settings
//! Step 14: downloads           → sys_downloads
//! Step 15: installations       → sys_installations
//! ```
//!
//! Steps 1–2 free the name `user_roles` before step 2 claims it.
//! Step 4 (recharges) is placed before step 3's DROP so that in PostgreSQL
//! the FK `recharges.user_id → users.id` is removed before `users` is dropped.

use crate::Result;
use sqlx::AnyPool;

/// Run all table rename data migrations in conflict-safe order.
pub(super) async fn migrate_table_renames(pool: &AnyPool, kind: &str) -> Result<()> {
    // Step 1–3: user_ domain — must follow the conflict-safe ordering above.
    copy_and_drop(pool, kind, "user_roles", "user_role_bindings").await;
    copy_and_drop(pool, kind, "roles", "user_roles").await;
    // Recharges (step 4) comes *before* users (step 3) to avoid a FK violation
    // in PostgreSQL when dropping `users` while `recharges.user_id` still
    // references it.
    copy_and_drop(pool, kind, "recharges", "user_recharges").await;
    copy_and_drop(pool, kind, "users", "user_accounts").await;

    // Steps 5–15: remaining tables (no inter-table name conflicts).
    copy_and_drop(pool, kind, "channels", "channel_providers").await;
    copy_and_drop(pool, kind, "abilities", "channel_abilities").await;
    copy_and_drop(pool, kind, "tokens", "user_api_keys").await;
    copy_and_drop(pool, kind, "prices", "billing_prices").await;
    copy_and_drop(pool, kind, "protocol_configs", "channel_protocol_configs").await;
    copy_and_drop(pool, kind, "tiered_pricing", "billing_tiered_prices").await;
    copy_and_drop(pool, kind, "exchange_rates", "billing_exchange_rates").await;
    copy_and_drop(pool, kind, "video_tasks", "router_video_tasks").await;
    copy_and_drop(pool, kind, "setting", "sys_settings").await;
    copy_and_drop(pool, kind, "downloads", "sys_downloads").await;
    copy_and_drop(pool, kind, "installations", "sys_installations").await;

    Ok(())
}

/// Copy all rows from `old_table` into `new_table`, then drop `old_table`.
///
/// The function is a no-op when:
/// - `old_table` does not exist (migration already completed or never needed).
/// - `new_table` already contains rows (copy already ran; skip to avoid
///   duplicates, but still drop the old table if it remains).
///
/// The DROP uses `CASCADE` on PostgreSQL to release any FK constraints that
/// still point at the old table name.  All errors are ignored so that a
/// partial failure does not abort the wider database initialisation.
async fn copy_and_drop(pool: &AnyPool, kind: &str, old_table: &str, new_table: &str) {
    if !table_exists(pool, kind, old_table).await {
        return;
    }

    // Guard: only copy when the destination is still empty to avoid duplicates.
    let new_count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {new_table}"))
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    if new_count == 0 {
        println!("[Rename] Migrating data: {old_table} → {new_table}");
        // INSERT INTO new SELECT * FROM old WHERE EXISTS (old table — already
        // confirmed above); errors are ignored (e.g. schema mismatch on
        // non-standard installs).
        let _ = sqlx::query(&format!(
            "INSERT INTO {new_table} SELECT * FROM {old_table}"
        ))
        .execute(pool)
        .await;
    }

    // Drop the old table.  PostgreSQL needs CASCADE to remove dependent FK
    // constraints; SQLite ignores the CASCADE keyword.
    let drop_sql = if kind == "postgres" {
        format!("DROP TABLE IF EXISTS {old_table} CASCADE")
    } else {
        format!("DROP TABLE IF EXISTS {old_table}")
    };
    let _ = sqlx::query(&drop_sql).execute(pool).await;
}

/// Returns `true` if `table_name` exists in the connected database.
async fn table_exists(pool: &AnyPool, kind: &str, table_name: &str) -> bool {
    let count: i64 = if kind == "sqlite" {
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?")
            .bind(table_name)
            .fetch_one(pool)
            .await
            .unwrap_or(0)
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM information_schema.tables \
             WHERE table_schema = 'public' AND table_name = $1",
        )
        .bind(table_name)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
    };
    count > 0
}
