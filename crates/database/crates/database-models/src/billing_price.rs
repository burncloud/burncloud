use crate::common::current_timestamp;
use burncloud_common::types::{Price, PriceInput};
use burncloud_database::{ph, phs, Database, Result};

pub struct BillingPriceModel;

impl BillingPriceModel {
    /// Get price for a model in a specific currency and region
    /// Falls back to USD if the requested currency is not found.
    ///
    /// `region = None` is normalized to `""` to match rows stored by `upsert()`.
    pub async fn get(
        db: &Database,
        model: &str,
        currency: &str,
        region: Option<&str>,
    ) -> Result<Option<Price>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        // upsert() normalises None → ""; queries must use the same convention.
        let region_key = region.unwrap_or("");

        // First try exact match (model, currency, region)
        let sql = format!(
            r#"SELECT id, model, currency, input_price, output_price,
                      cache_read_input_price, cache_creation_input_price,
                      batch_input_price, batch_output_price,
                      priority_input_price, priority_output_price,
                      audio_input_price, audio_output_price,
                      reasoning_price, embedding_price,
                      image_price, video_price, music_price,
                      source, region,
                      context_window, max_output_tokens,
                      supports_vision, supports_function_calling,
                      voices_pricing, video_pricing, asr_pricing, realtime_pricing, model_type,
                      synced_at, created_at, updated_at
               FROM billing_prices WHERE model = {} AND currency = {} AND region = {}"#,
            ph(is_postgres, 1),
            ph(is_postgres, 2),
            ph(is_postgres, 3)
        );

        let price = sqlx::query_as(&sql)
            .bind(model)
            .bind(currency)
            .bind(region_key)
            .fetch_optional(conn.pool())
            .await?;

        if price.is_some() {
            return Ok(price);
        }

        // Fallback to USD if different currency requested
        if currency != "USD" {
            let sql_usd = format!(
                r#"SELECT id, model, currency, input_price, output_price,
                          cache_read_input_price, cache_creation_input_price,
                          batch_input_price, batch_output_price,
                          priority_input_price, priority_output_price,
                          audio_input_price, audio_output_price,
                          reasoning_price, embedding_price,
                          image_price, video_price, music_price,
                          source, region,
                          context_window, max_output_tokens,
                          supports_vision, supports_function_calling,
                          voices_pricing, video_pricing, asr_pricing, realtime_pricing, model_type,
                          synced_at, created_at, updated_at
                   FROM billing_prices WHERE model = {} AND currency = 'USD' AND region = {}"#,
                ph(is_postgres, 1),
                ph(is_postgres, 2)
            );

            let usd_price = sqlx::query_as(&sql_usd)
                .bind(model)
                .bind(region_key)
                .fetch_optional(conn.pool())
                .await?;

            return Ok(usd_price);
        }

        Ok(None)
    }

    /// Get price for a model in a specific region
    /// With the new UNIQUE(model, region) constraint, each region has only one currency.
    /// Falls back to universal price (region="") if region-specific price not found.
    ///
    /// `region = None` is normalized to `""` to match rows stored by `upsert()`.
    pub async fn get_by_model_region(
        db: &Database,
        model: &str,
        region: Option<&str>,
    ) -> Result<Option<Price>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        // upsert() normalises None → ""; queries must use the same convention.
        let region_key = region.unwrap_or("");

        // First try exact match for the region
        let sql = format!(
            r#"SELECT id, model, currency, input_price, output_price,
                      cache_read_input_price, cache_creation_input_price,
                      batch_input_price, batch_output_price,
                      priority_input_price, priority_output_price,
                      audio_input_price, audio_output_price,
                      reasoning_price, embedding_price,
                      image_price, video_price, music_price,
                      source, region,
                      context_window, max_output_tokens,
                      supports_vision, supports_function_calling,
                      voices_pricing, video_pricing, asr_pricing, realtime_pricing, model_type,
                      synced_at, created_at, updated_at
               FROM billing_prices WHERE model = {} AND region = {}"#,
            ph(is_postgres, 1),
            ph(is_postgres, 2)
        );

        let price = sqlx::query_as(&sql)
            .bind(model)
            .bind(region_key)
            .fetch_optional(conn.pool())
            .await?;

        if price.is_some() {
            return Ok(price);
        }

        // Fallback to universal price (region = "") if region-specific price not found
        if !region_key.is_empty() {
            let sql_universal = format!(
                r#"SELECT id, model, currency, input_price, output_price,
                          cache_read_input_price, cache_creation_input_price,
                          batch_input_price, batch_output_price,
                          priority_input_price, priority_output_price,
                          audio_input_price, audio_output_price,
                          reasoning_price, embedding_price,
                          image_price, video_price, music_price,
                          source, region,
                          context_window, max_output_tokens,
                          supports_vision, supports_function_calling,
                          voices_pricing, video_pricing, asr_pricing, realtime_pricing, model_type,
                          synced_at, created_at, updated_at
                   FROM billing_prices WHERE model = {} AND region = ''"#,
                ph(is_postgres, 1)
            );

            let universal_price: Option<Price> = sqlx::query_as(&sql_universal)
                .bind(model)
                .fetch_optional(conn.pool())
                .await?;

            return Ok(universal_price);
        }

        Ok(None)
    }

    /// Get all prices for a model across all currencies
    pub async fn get_all_currencies(
        db: &Database,
        model: &str,
        region: Option<&str>,
    ) -> Result<Vec<Price>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = if is_postgres {
            format!(
                r#"SELECT id, model, currency, input_price, output_price,
                      cache_read_input_price, cache_creation_input_price,
                      batch_input_price, batch_output_price,
                      priority_input_price, priority_output_price,
                      audio_input_price, audio_output_price,
                      reasoning_price, embedding_price,
                      image_price, video_price, music_price,
                      source, region,
                      context_window, max_output_tokens,
                      supports_vision, supports_function_calling,
                      voices_pricing, video_pricing, asr_pricing, realtime_pricing, model_type,
                      synced_at, created_at, updated_at
               FROM billing_prices WHERE model = {} AND region IS NOT DISTINCT FROM {}
               ORDER BY currency"#,
                ph(is_postgres, 1),
                ph(is_postgres, 2)
            )
        } else {
            format!(
                r#"SELECT id, model, currency, input_price, output_price,
                      cache_read_input_price, cache_creation_input_price,
                      batch_input_price, batch_output_price,
                      priority_input_price, priority_output_price,
                      audio_input_price, audio_output_price,
                      reasoning_price, embedding_price,
                      image_price, video_price, music_price,
                      source, region,
                      context_window, max_output_tokens,
                      supports_vision, supports_function_calling,
                      voices_pricing, video_pricing, asr_pricing, realtime_pricing, model_type,
                      synced_at, created_at, updated_at
               FROM billing_prices WHERE model = {} AND (region = {} OR (region IS NULL AND {} IS NULL))
               ORDER BY currency"#,
                ph(is_postgres, 1),
                ph(is_postgres, 2),
                ph(is_postgres, 3)
            )
        };

        let prices = if is_postgres {
            sqlx::query_as(&sql)
                .bind(model)
                .bind(region)
                .fetch_all(conn.pool())
                .await?
        } else {
            sqlx::query_as(&sql)
                .bind(model)
                .bind(region)
                .bind(region)
                .fetch_all(conn.pool())
                .await?
        };

        Ok(prices)
    }

    /// List all prices with pagination
    pub async fn list(
        db: &Database,
        limit: i32,
        offset: i32,
        currency: Option<&str>,
        region: Option<&str>,
    ) -> Result<Vec<Price>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let base_select = r#"SELECT id, model, currency, input_price, output_price,
                              cache_read_input_price, cache_creation_input_price,
                              batch_input_price, batch_output_price,
                              priority_input_price, priority_output_price,
                              audio_input_price, audio_output_price,
                              reasoning_price, embedding_price, image_price, video_price, music_price,
                              source, region,
                              context_window, max_output_tokens,
                              supports_vision, supports_function_calling,
                              voices_pricing, video_pricing, asr_pricing, realtime_pricing, model_type,
                              synced_at, created_at, updated_at"#;

        let prices = match (currency, region) {
            (Some(curr), Some(reg)) => {
                // Filter by both currency and region
                let sql = if is_postgres {
                    format!(
                        r#"{} FROM billing_prices WHERE currency = {} AND region IS NOT DISTINCT FROM {}
                       ORDER BY model LIMIT {} OFFSET {}"#,
                        base_select,
                        ph(is_postgres, 1),
                        ph(is_postgres, 2),
                        ph(is_postgres, 3),
                        ph(is_postgres, 4)
                    )
                } else {
                    format!(
                        r#"{} FROM billing_prices WHERE currency = {} AND (region = {} OR (region IS NULL AND {} IS NULL))
                       ORDER BY model LIMIT {} OFFSET {}"#,
                        base_select,
                        ph(is_postgres, 1),
                        ph(is_postgres, 2),
                        ph(is_postgres, 3),
                        ph(is_postgres, 4),
                        ph(is_postgres, 5)
                    )
                };
                if is_postgres {
                    sqlx::query_as(&sql)
                        .bind(curr)
                        .bind(reg)
                        .bind(limit)
                        .bind(offset)
                        .fetch_all(conn.pool())
                        .await?
                } else {
                    sqlx::query_as(&sql)
                        .bind(curr)
                        .bind(reg)
                        .bind(reg)
                        .bind(limit)
                        .bind(offset)
                        .fetch_all(conn.pool())
                        .await?
                }
            }
            (Some(curr), None) => {
                // Filter by currency only
                let sql = format!(
                    r#"{} FROM billing_prices WHERE currency = {}
                       ORDER BY model LIMIT {} OFFSET {}"#,
                    base_select,
                    ph(is_postgres, 1),
                    ph(is_postgres, 2),
                    ph(is_postgres, 3)
                );
                sqlx::query_as(&sql)
                    .bind(curr)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(conn.pool())
                    .await?
            }
            (None, Some(reg)) => {
                // Filter by region only
                let sql = if is_postgres {
                    format!(
                        r#"{} FROM billing_prices WHERE region IS NOT DISTINCT FROM {}
                       ORDER BY model, currency LIMIT {} OFFSET {}"#,
                        base_select,
                        ph(is_postgres, 1),
                        ph(is_postgres, 2),
                        ph(is_postgres, 3)
                    )
                } else {
                    format!(
                        r#"{} FROM billing_prices WHERE (region = {} OR (region IS NULL AND {} IS NULL))
                       ORDER BY model, currency LIMIT {} OFFSET {}"#,
                        base_select,
                        ph(is_postgres, 1),
                        ph(is_postgres, 2),
                        ph(is_postgres, 3),
                        ph(is_postgres, 4)
                    )
                };
                if is_postgres {
                    sqlx::query_as(&sql)
                        .bind(reg)
                        .bind(limit)
                        .bind(offset)
                        .fetch_all(conn.pool())
                        .await?
                } else {
                    sqlx::query_as(&sql)
                        .bind(reg)
                        .bind(reg)
                        .bind(limit)
                        .bind(offset)
                        .fetch_all(conn.pool())
                        .await?
                }
            }
            (None, None) => {
                // No filters
                let sql = format!(
                    r#"{} FROM billing_prices ORDER BY model, currency LIMIT {} OFFSET {}"#,
                    base_select,
                    ph(is_postgres, 1),
                    ph(is_postgres, 2)
                );
                sqlx::query_as(&sql)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(conn.pool())
                    .await?
            }
        };

        Ok(prices)
    }

    /// Create or update a price (upsert)
    pub async fn upsert(db: &Database, input: &PriceInput) -> Result<()> {
        let conn = db.get_connection()?;
        let now = current_timestamp();
        let is_postgres = db.kind() == "postgres";

        let sql = format!(
            r#"
            INSERT INTO billing_prices (
                model, currency, input_price, output_price,
                cache_read_input_price, cache_creation_input_price,
                batch_input_price, batch_output_price,
                priority_input_price, priority_output_price,
                audio_input_price, audio_output_price,
                reasoning_price, embedding_price,
                image_price, video_price, music_price,
                source, region,
                context_window, max_output_tokens,
                supports_vision, supports_function_calling,
                voices_pricing, video_pricing, asr_pricing, realtime_pricing, model_type,
                synced_at, created_at, updated_at
            ) VALUES ({})
            ON CONFLICT(model, region) DO UPDATE SET
                currency = EXCLUDED.currency,
                input_price = EXCLUDED.input_price,
                output_price = EXCLUDED.output_price,
                cache_read_input_price = EXCLUDED.cache_read_input_price,
                cache_creation_input_price = EXCLUDED.cache_creation_input_price,
                batch_input_price = EXCLUDED.batch_input_price,
                batch_output_price = EXCLUDED.batch_output_price,
                priority_input_price = EXCLUDED.priority_input_price,
                priority_output_price = EXCLUDED.priority_output_price,
                audio_input_price = EXCLUDED.audio_input_price,
                audio_output_price = EXCLUDED.audio_output_price,
                reasoning_price = EXCLUDED.reasoning_price,
                embedding_price = EXCLUDED.embedding_price,
                image_price = EXCLUDED.image_price,
                video_price = EXCLUDED.video_price,
                music_price = EXCLUDED.music_price,
                source = EXCLUDED.source,
                context_window = EXCLUDED.context_window,
                max_output_tokens = EXCLUDED.max_output_tokens,
                supports_vision = EXCLUDED.supports_vision,
                supports_function_calling = EXCLUDED.supports_function_calling,
                voices_pricing = EXCLUDED.voices_pricing,
                video_pricing = EXCLUDED.video_pricing,
                asr_pricing = EXCLUDED.asr_pricing,
                realtime_pricing = EXCLUDED.realtime_pricing,
                model_type = EXCLUDED.model_type,
                synced_at = EXCLUDED.synced_at,
                updated_at = EXCLUDED.updated_at
            "#,
            phs(is_postgres, 31)
        );

        // Normalize region: None → "" to ensure UNIQUE(model, region) works correctly.
        // SQLite treats NULL != NULL, so NULL regions can't deduplicate via ON CONFLICT.
        let region = input.region.as_deref().unwrap_or("").to_string();

        sqlx::query(&sql)
            .bind(&input.model)
            .bind(&input.currency)
            .bind(input.input_price)
            .bind(input.output_price)
            .bind(input.cache_read_input_price)
            .bind(input.cache_creation_input_price)
            .bind(input.batch_input_price)
            .bind(input.batch_output_price)
            .bind(input.priority_input_price)
            .bind(input.priority_output_price)
            .bind(input.audio_input_price)
            .bind(input.audio_output_price)
            .bind(input.reasoning_price)
            .bind(input.embedding_price)
            .bind(input.image_price)
            .bind(input.video_price)
            .bind(input.music_price)
            .bind(&input.source)
            .bind(&region)
            .bind(input.context_window)
            .bind(input.max_output_tokens)
            .bind(input.supports_vision.map(|v| v as i32))
            .bind(input.supports_function_calling.map(|v| v as i32))
            .bind(&input.voices_pricing)
            .bind(&input.video_pricing)
            .bind(&input.asr_pricing)
            .bind(&input.realtime_pricing)
            .bind(&input.model_type)
            .bind(now) // synced_at
            .bind(now) // created_at
            .bind(now) // updated_at
            .execute(conn.pool())
            .await?;

        Ok(())
    }

    /// Batch upsert multiple prices in a single transaction.
    ///
    /// For LiteLLM sync (~4000 models), this reduces ~4000 individual SQLite
    /// round-trips (each with an implicit fsync) to a single transaction commit.
    /// Typical speedup: 50-100x for SQLite.
    ///
    /// Returns the number of rows upserted, counting only successes.
    /// On any row error the transaction is rolled back and the error is propagated.
    pub async fn batch_upsert(db: &Database, inputs: &[PriceInput]) -> Result<usize> {
        if inputs.is_empty() {
            return Ok(0);
        }

        let conn = db.get_connection()?;
        let now = current_timestamp();
        let is_postgres = db.kind() == "postgres";

        let sql = format!(
            r#"
            INSERT INTO billing_prices (
                model, currency, input_price, output_price,
                cache_read_input_price, cache_creation_input_price,
                batch_input_price, batch_output_price,
                priority_input_price, priority_output_price,
                audio_input_price, audio_output_price,
                reasoning_price, embedding_price,
                image_price, video_price, music_price,
                source, region,
                context_window, max_output_tokens,
                supports_vision, supports_function_calling,
                voices_pricing, video_pricing, asr_pricing, realtime_pricing, model_type,
                synced_at, created_at, updated_at
            ) VALUES ({})
            ON CONFLICT(model, region) DO UPDATE SET
                currency = EXCLUDED.currency,
                input_price = EXCLUDED.input_price,
                output_price = EXCLUDED.output_price,
                cache_read_input_price = EXCLUDED.cache_read_input_price,
                cache_creation_input_price = EXCLUDED.cache_creation_input_price,
                batch_input_price = EXCLUDED.batch_input_price,
                batch_output_price = EXCLUDED.batch_output_price,
                priority_input_price = EXCLUDED.priority_input_price,
                priority_output_price = EXCLUDED.priority_output_price,
                audio_input_price = EXCLUDED.audio_input_price,
                audio_output_price = EXCLUDED.audio_output_price,
                reasoning_price = EXCLUDED.reasoning_price,
                embedding_price = EXCLUDED.embedding_price,
                image_price = EXCLUDED.image_price,
                video_price = EXCLUDED.video_price,
                music_price = EXCLUDED.music_price,
                source = EXCLUDED.source,
                context_window = EXCLUDED.context_window,
                max_output_tokens = EXCLUDED.max_output_tokens,
                supports_vision = EXCLUDED.supports_vision,
                supports_function_calling = EXCLUDED.supports_function_calling,
                voices_pricing = EXCLUDED.voices_pricing,
                video_pricing = EXCLUDED.video_pricing,
                asr_pricing = EXCLUDED.asr_pricing,
                realtime_pricing = EXCLUDED.realtime_pricing,
                model_type = EXCLUDED.model_type,
                synced_at = EXCLUDED.synced_at,
                updated_at = EXCLUDED.updated_at
            "#,
            phs(is_postgres, 31)
        );

        let mut tx = conn.pool().begin().await?;
        let mut count = 0usize;

        for input in inputs {
            // Normalize region: None → "" (same logic as upsert)
            let region = input.region.as_deref().unwrap_or("").to_string();

            sqlx::query(&sql)
                .bind(&input.model)
                .bind(&input.currency)
                .bind(input.input_price)
                .bind(input.output_price)
                .bind(input.cache_read_input_price)
                .bind(input.cache_creation_input_price)
                .bind(input.batch_input_price)
                .bind(input.batch_output_price)
                .bind(input.priority_input_price)
                .bind(input.priority_output_price)
                .bind(input.audio_input_price)
                .bind(input.audio_output_price)
                .bind(input.reasoning_price)
                .bind(input.embedding_price)
                .bind(input.image_price)
                .bind(input.video_price)
                .bind(input.music_price)
                .bind(&input.source)
                .bind(&region)
                .bind(input.context_window)
                .bind(input.max_output_tokens)
                .bind(input.supports_vision.map(|v| v as i32))
                .bind(input.supports_function_calling.map(|v| v as i32))
                .bind(&input.voices_pricing)
                .bind(&input.video_pricing)
                .bind(&input.asr_pricing)
                .bind(&input.realtime_pricing)
                .bind(&input.model_type)
                .bind(now) // synced_at
                .bind(now) // created_at
                .bind(now) // updated_at
                .execute(&mut *tx)
                .await?;

            count += 1;
        }

        tx.commit().await?;
        Ok(count)
    }

    /// Delete a price entry
    pub async fn delete(
        db: &Database,
        model: &str,
        currency: &str,
        region: Option<&str>,
    ) -> Result<bool> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let result = match region {
            Some(r) => {
                let sql =
                    format!(
                    "DELETE FROM billing_prices WHERE model = {} AND currency = {} AND region = {}",
                    ph(is_postgres, 1), ph(is_postgres, 2), ph(is_postgres, 3)
                );
                sqlx::query(&sql)
                    .bind(model)
                    .bind(currency)
                    .bind(r)
                    .execute(conn.pool())
                    .await?
            }
            None => {
                let sql = format!(
                    "DELETE FROM billing_prices WHERE model = {} AND currency = {} AND region IS NULL",
                    ph(is_postgres, 1), ph(is_postgres, 2)
                );
                sqlx::query(&sql)
                    .bind(model)
                    .bind(currency)
                    .execute(conn.pool())
                    .await?
            }
        };

        Ok(result.rows_affected() > 0)
    }

    /// Delete all prices for a model
    pub async fn delete_all_for_model(db: &Database, model: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = format!(
            "DELETE FROM billing_prices WHERE model = {}",
            ph(is_postgres, 1)
        );

        sqlx::query(&sql).bind(model).execute(conn.pool()).await?;

        Ok(())
    }

    /// Delete price for a model in a specific region
    /// Returns the number of deleted rows
    pub async fn delete_by_region(db: &Database, model: &str, region: &str) -> Result<u64> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = format!(
            "DELETE FROM billing_prices WHERE model = {} AND region = {}",
            ph(is_postgres, 1),
            ph(is_postgres, 2)
        );

        let result = sqlx::query(&sql)
            .bind(model)
            .bind(region)
            .execute(conn.pool())
            .await?;

        Ok(result.rows_affected())
    }

    /// Calculate cost for a request using Price
    /// Returns cost in nanodollars (i64, 9 decimal precision)
    pub fn calculate_cost(price: &Price, prompt_tokens: u64, completion_tokens: u64) -> i64 {
        // Use i128 intermediate to prevent overflow
        let input_cost = (prompt_tokens as i128 * price.input_price as i128) / 1_000_000;
        let output_cost = (completion_tokens as i128 * price.output_price as i128) / 1_000_000;
        (input_cost + output_cost) as i64
    }
}
