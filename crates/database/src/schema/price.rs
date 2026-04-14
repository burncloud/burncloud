//! Price-related data migrations.
//!
//! Handles:
//! - Renaming `prices_v2` → `prices`
//! - Cleaning up temporary migration tables
//! - Migrating data from `prices_deprecated` into the canonical `prices` table
//! - Converting REAL-typed price values to nanodollars (SQLite only)
//! - Converting small-integer (old USD dollar format) prices to nanodollars (SQLite only)
//! - Normalising NULL region values and deduplicating rows

use crate::Result;
use sqlx::AnyPool;

/// Run all price-related data migrations.
pub(super) async fn migrate_prices(pool: &AnyPool, kind: &str) -> Result<()> {
    migrate_prices_v2(pool, kind).await?;
    cleanup_temp_tables(pool, kind).await?;
    migrate_prices_deprecated(pool, kind).await?;
    fix_real_prices(pool, kind).await?;
    fix_small_int_prices(pool, kind).await?;
    normalise_regions(pool).await?;
    Ok(())
}

/// Rename `prices_v2` → `prices`, preserving existing `prices` as `prices_deprecated`.
async fn migrate_prices_v2(pool: &AnyPool, kind: &str) -> Result<()> {
    let prices_v2_exists: bool = if kind == "sqlite" {
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='prices_v2'",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);
        count > 0
    } else {
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM information_schema.tables WHERE table_name = 'prices_v2'",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);
        count > 0
    };

    if !prices_v2_exists {
        return Ok(());
    }

    println!("Migrating prices_v2 to prices table...");

    let old_prices_exists: bool = if kind == "sqlite" {
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='prices'",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);
        count > 0
    } else {
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM information_schema.tables WHERE table_name = 'prices'",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);
        count > 0
    };

    if old_prices_exists {
        let _ = sqlx::query("DROP TABLE IF EXISTS prices_deprecated")
            .execute(pool)
            .await;
        let _ = sqlx::query("ALTER TABLE prices RENAME TO prices_deprecated")
            .execute(pool)
            .await;
        println!("  Renamed old 'prices' table to 'prices_deprecated'");
    }

    let _ = sqlx::query("ALTER TABLE prices_v2 RENAME TO prices")
        .execute(pool)
        .await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_prices_model ON prices(model)")
        .execute(pool)
        .await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_prices_model_region ON prices(model, region)",
    )
    .execute(pool)
    .await;
    println!("  Renamed 'prices_v2' to 'prices'");
    Ok(())
}

/// Drop temporary migration tables left from older migration attempts.
async fn cleanup_temp_tables(pool: &AnyPool, kind: &str) -> Result<()> {
    let temp_tables = ["prices_v2_new", "tiered_pricing_new", "exchange_rates_new"];
    for table in temp_tables {
        if kind == "sqlite" {
            let exists: i64 = sqlx::query_scalar(&format!(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='{table}'"
            ))
            .fetch_one(pool)
            .await
            .unwrap_or(0);

            if exists > 0 {
                let _ = sqlx::query(&format!("DROP TABLE {table}"))
                    .execute(pool)
                    .await;
                println!("  Dropped temporary table '{table}'");
            }
        } else {
            let _ = sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
                .execute(pool)
                .await;
        }
    }
    Ok(())
}

/// Copy data from `prices_deprecated` into `prices` when `prices` is empty.
async fn migrate_prices_deprecated(pool: &AnyPool, kind: &str) -> Result<()> {
    let prices_count: i64 = sqlx::query_scalar("SELECT count(*) FROM prices")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let deprecated_exists: bool = if kind == "sqlite" {
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='prices_deprecated'",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);
        count > 0
    } else {
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM information_schema.tables \
             WHERE table_name = 'prices_deprecated'",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);
        count > 0
    };

    if prices_count != 0 || !deprecated_exists {
        return Ok(());
    }

    let now = crate::schema::current_timestamp();
    let migrate_sql = match kind {
        "sqlite" => {
            r#"
            INSERT INTO prices (
                model, currency, input_price, output_price,
                cache_read_input_price, cache_creation_input_price,
                batch_input_price, batch_output_price,
                priority_input_price, priority_output_price,
                audio_input_price, source, region,
                created_at, updated_at
            )
            SELECT
                model, 'USD',
                CAST(ROUND(input_price * 1000000000) AS BIGINT),
                CAST(ROUND(output_price * 1000000000) AS BIGINT),
                CASE WHEN cache_read_price IS NOT NULL THEN CAST(ROUND(cache_read_price * 1000000000) AS BIGINT) END,
                CASE WHEN cache_creation_price IS NOT NULL THEN CAST(ROUND(cache_creation_price * 1000000000) AS BIGINT) END,
                CASE WHEN batch_input_price IS NOT NULL THEN CAST(ROUND(batch_input_price * 1000000000) AS BIGINT) END,
                CASE WHEN batch_output_price IS NOT NULL THEN CAST(ROUND(batch_output_price * 1000000000) AS BIGINT) END,
                CASE WHEN priority_input_price IS NOT NULL THEN CAST(ROUND(priority_input_price * 1000000000) AS BIGINT) END,
                CASE WHEN priority_output_price IS NOT NULL THEN CAST(ROUND(priority_output_price * 1000000000) AS BIGINT) END,
                CASE WHEN audio_input_price IS NOT NULL THEN CAST(ROUND(audio_input_price * 1000000000) AS BIGINT) END,
                NULL, NULL,
                ?, ?
            FROM prices_deprecated
            "#
        }
        "postgres" => {
            r#"
            INSERT INTO prices (
                model, currency, input_price, output_price,
                cache_read_input_price, cache_creation_input_price,
                batch_input_price, batch_output_price,
                priority_input_price, priority_output_price,
                audio_input_price, source, region,
                created_at, updated_at
            )
            SELECT
                model, 'USD',
                ROUND(input_price * 1000000000)::BIGINT,
                ROUND(output_price * 1000000000)::BIGINT,
                CASE WHEN cache_read_price IS NOT NULL THEN ROUND(cache_read_price * 1000000000)::BIGINT END,
                CASE WHEN cache_creation_price IS NOT NULL THEN ROUND(cache_creation_price * 1000000000)::BIGINT END,
                CASE WHEN batch_input_price IS NOT NULL THEN ROUND(batch_input_price * 1000000000)::BIGINT END,
                CASE WHEN batch_output_price IS NOT NULL THEN ROUND(batch_output_price * 1000000000)::BIGINT END,
                CASE WHEN priority_input_price IS NOT NULL THEN ROUND(priority_input_price * 1000000000)::BIGINT END,
                CASE WHEN priority_output_price IS NOT NULL THEN ROUND(priority_output_price * 1000000000)::BIGINT END,
                CASE WHEN audio_input_price IS NOT NULL THEN ROUND(audio_input_price * 1000000000)::BIGINT END,
                NULL, NULL,
                $1, $2
            FROM prices_deprecated
            ON CONFLICT (model, region) DO NOTHING
            "#
        }
        _ => return Ok(()),
    };

    let _ = sqlx::query(migrate_sql)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await;
    println!("  Migrated data from prices_deprecated to prices");
    Ok(())
}

/// Convert REAL-typed price columns to nanodollars (SQLite only).
///
/// Some rows were inserted as dollar floats before the nanodollar migration.
async fn fix_real_prices(pool: &AnyPool, kind: &str) -> Result<()> {
    if kind != "sqlite" {
        return Ok(());
    }

    let real_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM prices WHERE typeof(input_price) = 'real'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if real_count == 0 {
        return Ok(());
    }

    println!("Fixing {real_count} rows with REAL-typed prices (converting dollars to nanodollars)...");
    let price_cols = price_column_names();
    let set_clauses = price_cols
        .iter()
        .map(|col| {
            format!(
                "{col} = CASE WHEN typeof({col}) = 'real' \
                 THEN CAST(ROUND({col} * 1000000000) AS INTEGER) \
                 ELSE {col} END"
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let update_sql =
        format!("UPDATE prices SET {set_clauses} WHERE typeof(input_price) = 'real'");
    let _ = sqlx::query(&update_sql).execute(pool).await;
    println!("  Converted dollar-format prices to nanodollar format");
    Ok(())
}

/// Convert small-integer (old USD dollar format) prices to nanodollars (SQLite only).
///
/// Threshold: any non-zero price < 10_000 is considered old USD format.
async fn fix_small_int_prices(pool: &AnyPool, kind: &str) -> Result<()> {
    if kind != "sqlite" {
        return Ok(());
    }

    let small_int_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM prices \
         WHERE (input_price > 0 AND input_price < 10000) \
            OR (output_price > 0 AND output_price < 10000)",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if small_int_count == 0 {
        return Ok(());
    }

    println!(
        "Fixing {small_int_count} rows with small-integer prices \
         (old USD dollar format -> nanodollars)..."
    );
    let price_cols = price_column_names();
    let set_clauses = price_cols
        .iter()
        .map(|col| {
            format!(
                "{col} = CASE WHEN {col} IS NOT NULL AND {col} > 0 \
                 AND {col} < 10000 THEN {col} * 1000000000 ELSE {col} END"
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let update_sql = format!(
        "UPDATE prices SET {set_clauses} WHERE \
         (input_price > 0 AND input_price < 10000) \
         OR (output_price > 0 AND output_price < 10000)"
    );
    let _ = sqlx::query(&update_sql).execute(pool).await;
    println!("  Converted small-integer USD prices to nanodollar format");
    Ok(())
}

/// Normalise NULL region values and deduplicate rows in `prices`.
///
/// SQLite UNIQUE(model, region) does NOT deduplicate NULLs (SQL standard:
/// NULL != NULL).  Normalise NULL → '' then remove duplicate rows.
async fn normalise_regions(pool: &AnyPool) -> Result<()> {
    let _ = sqlx::query(
        "DELETE FROM prices
         WHERE region IS NULL
           AND EXISTS (
               SELECT 1 FROM prices p2
               WHERE p2.model = prices.model
                 AND p2.currency = prices.currency
                 AND p2.region = ''
           )",
    )
    .execute(pool)
    .await;

    let _ = sqlx::query("UPDATE prices SET region = '' WHERE region IS NULL")
        .execute(pool)
        .await;

    let dedup_result = sqlx::query(
        "DELETE FROM prices WHERE id NOT IN (
             SELECT MAX(id) FROM prices GROUP BY model, region
         )",
    )
    .execute(pool)
    .await;
    if let Ok(r) = dedup_result {
        let removed = r.rows_affected();
        if removed > 0 {
            println!("  Removed {removed} duplicate price rows");
        }
    }
    Ok(())
}

/// Return the list of price column names used for format-conversion updates.
fn price_column_names() -> &'static [&'static str] {
    &[
        "input_price",
        "output_price",
        "cache_read_input_price",
        "cache_creation_input_price",
        "batch_input_price",
        "batch_output_price",
        "priority_input_price",
        "priority_output_price",
        "audio_input_price",
        "audio_output_price",
        "reasoning_price",
        "embedding_price",
        "image_price",
        "video_price",
    ]
}
