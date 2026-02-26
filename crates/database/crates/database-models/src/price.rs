use crate::common::current_timestamp;
use burncloud_common::types::{Price, PriceInput};
use burncloud_database::{Database, Result};

pub struct PriceModel;

impl PriceModel {
    /// Get price for a model in a specific currency and region
    /// Falls back to USD if the requested currency is not found
    pub async fn get(
        db: &Database,
        model: &str,
        currency: &str,
        region: Option<&str>,
    ) -> Result<Option<Price>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        // First try exact match (model, currency, region)
        let sql = if is_postgres {
            r#"SELECT id, model, currency, input_price, output_price,
                      cache_read_input_price, cache_creation_input_price,
                      batch_input_price, batch_output_price,
                      priority_input_price, priority_output_price,
                      audio_input_price, source, region,
                      context_window, max_output_tokens,
                      supports_vision, supports_function_calling,
                      synced_at, created_at, updated_at
               FROM prices WHERE model = $1 AND currency = $2 AND region IS NOT DISTINCT FROM $3"#
        } else {
            r#"SELECT id, model, currency, input_price, output_price,
                      cache_read_input_price, cache_creation_input_price,
                      batch_input_price, batch_output_price,
                      priority_input_price, priority_output_price,
                      audio_input_price, source, region,
                      context_window, max_output_tokens,
                      supports_vision, supports_function_calling,
                      synced_at, created_at, updated_at
               FROM prices WHERE model = ? AND currency = ? AND (region = ? OR (region IS NULL AND ? IS NULL))"#
        };

        let price = if is_postgres {
            sqlx::query_as(sql)
                .bind(model)
                .bind(currency)
                .bind(region)
                .fetch_optional(conn.pool())
                .await?
        } else {
            sqlx::query_as(sql)
                .bind(model)
                .bind(currency)
                .bind(region)
                .bind(region)
                .fetch_optional(conn.pool())
                .await?
        };

        if price.is_some() {
            return Ok(price);
        }

        // Fallback to USD if different currency requested
        if currency != "USD" {
            let sql_usd = if is_postgres {
                r#"SELECT id, model, currency, input_price, output_price,
                          cache_read_input_price, cache_creation_input_price,
                          batch_input_price, batch_output_price,
                          priority_input_price, priority_output_price,
                          audio_input_price, source, region,
                          context_window, max_output_tokens,
                          supports_vision, supports_function_calling,
                          synced_at, created_at, updated_at
                   FROM prices WHERE model = $1 AND currency = 'USD' AND region IS NOT DISTINCT FROM $2"#
            } else {
                r#"SELECT id, model, currency, input_price, output_price,
                          cache_read_input_price, cache_creation_input_price,
                          batch_input_price, batch_output_price,
                          priority_input_price, priority_output_price,
                          audio_input_price, source, region,
                          context_window, max_output_tokens,
                          supports_vision, supports_function_calling,
                          synced_at, created_at, updated_at
                   FROM prices WHERE model = ? AND currency = 'USD' AND (region = ? OR (region IS NULL AND ? IS NULL))"#
            };

            let usd_price = if is_postgres {
                sqlx::query_as(sql_usd)
                    .bind(model)
                    .bind(region)
                    .fetch_optional(conn.pool())
                    .await?
            } else {
                sqlx::query_as(sql_usd)
                    .bind(model)
                    .bind(region)
                    .bind(region)
                    .fetch_optional(conn.pool())
                    .await?
            };

            return Ok(usd_price);
        }

        Ok(None)
    }

    /// Get price for a model in a specific region
    /// With the new UNIQUE(model, region) constraint, each region has only one currency.
    /// Falls back to universal price (region=NULL) if region-specific price not found.
    pub async fn get_by_model_region(
        db: &Database,
        model: &str,
        region: Option<&str>,
    ) -> Result<Option<Price>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        // First try exact match for the region
        let sql = if is_postgres {
            r#"SELECT id, model, currency, input_price, output_price,
                      cache_read_input_price, cache_creation_input_price,
                      batch_input_price, batch_output_price,
                      priority_input_price, priority_output_price,
                      audio_input_price, source, region,
                      context_window, max_output_tokens,
                      supports_vision, supports_function_calling,
                      synced_at, created_at, updated_at
               FROM prices WHERE model = $1 AND region IS NOT DISTINCT FROM $2"#
        } else {
            r#"SELECT id, model, currency, input_price, output_price,
                      cache_read_input_price, cache_creation_input_price,
                      batch_input_price, batch_output_price,
                      priority_input_price, priority_output_price,
                      audio_input_price, source, region,
                      context_window, max_output_tokens,
                      supports_vision, supports_function_calling,
                      synced_at, created_at, updated_at
               FROM prices WHERE model = ? AND (region = ? OR (region IS NULL AND ? IS NULL))"#
        };

        let price = if is_postgres {
            sqlx::query_as(sql)
                .bind(model)
                .bind(region)
                .fetch_optional(conn.pool())
                .await?
        } else {
            sqlx::query_as(sql)
                .bind(model)
                .bind(region)
                .bind(region)
                .fetch_optional(conn.pool())
                .await?
        };

        if price.is_some() {
            return Ok(price);
        }

        // Fallback to universal price (region = NULL) if region-specific price not found
        if region.is_some() {
            let sql_universal = if is_postgres {
                r#"SELECT id, model, currency, input_price, output_price,
                          cache_read_input_price, cache_creation_input_price,
                          batch_input_price, batch_output_price,
                          priority_input_price, priority_output_price,
                          audio_input_price, source, region,
                          context_window, max_output_tokens,
                          supports_vision, supports_function_calling,
                          synced_at, created_at, updated_at
                   FROM prices WHERE model = $1 AND region IS NULL"#
            } else {
                r#"SELECT id, model, currency, input_price, output_price,
                          cache_read_input_price, cache_creation_input_price,
                          batch_input_price, batch_output_price,
                          priority_input_price, priority_output_price,
                          audio_input_price, source, region,
                          context_window, max_output_tokens,
                          supports_vision, supports_function_calling,
                          synced_at, created_at, updated_at
                   FROM prices WHERE model = ? AND region IS NULL"#
            };

            let universal_price: Option<Price> = sqlx::query_as(sql_universal)
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
            r#"SELECT id, model, currency, input_price, output_price,
                      cache_read_input_price, cache_creation_input_price,
                      batch_input_price, batch_output_price,
                      priority_input_price, priority_output_price,
                      audio_input_price, source, region,
                      context_window, max_output_tokens,
                      supports_vision, supports_function_calling,
                      synced_at, created_at, updated_at
               FROM prices WHERE model = $1 AND region IS NOT DISTINCT FROM $2
               ORDER BY currency"#
        } else {
            r#"SELECT id, model, currency, input_price, output_price,
                      cache_read_input_price, cache_creation_input_price,
                      batch_input_price, batch_output_price,
                      priority_input_price, priority_output_price,
                      audio_input_price, source, region,
                      context_window, max_output_tokens,
                      supports_vision, supports_function_calling,
                      synced_at, created_at, updated_at
               FROM prices WHERE model = ? AND (region = ? OR (region IS NULL AND ? IS NULL))
               ORDER BY currency"#
        };

        let prices = if is_postgres {
            sqlx::query_as(sql)
                .bind(model)
                .bind(region)
                .fetch_all(conn.pool())
                .await?
        } else {
            sqlx::query_as(sql)
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

        let prices = match (currency, region) {
            (Some(curr), Some(reg)) => {
                // Filter by both currency and region
                let sql = if is_postgres {
                    r#"SELECT id, model, currency, input_price, output_price,
                              cache_read_input_price, cache_creation_input_price,
                              batch_input_price, batch_output_price,
                              priority_input_price, priority_output_price,
                              audio_input_price, source, region,
                              context_window, max_output_tokens,
                              supports_vision, supports_function_calling,
                              synced_at, created_at, updated_at
                       FROM prices WHERE currency = $1 AND region IS NOT DISTINCT FROM $2
                       ORDER BY model LIMIT $3 OFFSET $4"#
                } else {
                    r#"SELECT id, model, currency, input_price, output_price,
                              cache_read_input_price, cache_creation_input_price,
                              batch_input_price, batch_output_price,
                              priority_input_price, priority_output_price,
                              audio_input_price, source, region,
                              context_window, max_output_tokens,
                              supports_vision, supports_function_calling,
                              synced_at, created_at, updated_at
                       FROM prices WHERE currency = ? AND (region = ? OR (region IS NULL AND ? IS NULL))
                       ORDER BY model LIMIT ? OFFSET ?"#
                };
                if is_postgres {
                    sqlx::query_as(sql)
                        .bind(curr)
                        .bind(reg)
                        .bind(limit)
                        .bind(offset)
                        .fetch_all(conn.pool())
                        .await?
                } else {
                    sqlx::query_as(sql)
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
                let sql = if is_postgres {
                    r#"SELECT id, model, currency, input_price, output_price,
                              cache_read_input_price, cache_creation_input_price,
                              batch_input_price, batch_output_price,
                              priority_input_price, priority_output_price,
                              audio_input_price, source, region,
                              context_window, max_output_tokens,
                              supports_vision, supports_function_calling,
                              synced_at, created_at, updated_at
                       FROM prices WHERE currency = $1
                       ORDER BY model LIMIT $2 OFFSET $3"#
                } else {
                    r#"SELECT id, model, currency, input_price, output_price,
                              cache_read_input_price, cache_creation_input_price,
                              batch_input_price, batch_output_price,
                              priority_input_price, priority_output_price,
                              audio_input_price, source, region,
                              context_window, max_output_tokens,
                              supports_vision, supports_function_calling,
                              synced_at, created_at, updated_at
                       FROM prices WHERE currency = ?
                       ORDER BY model LIMIT ? OFFSET ?"#
                };
                sqlx::query_as(sql)
                    .bind(curr)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(conn.pool())
                    .await?
            }
            (None, Some(reg)) => {
                // Filter by region only
                let sql = if is_postgres {
                    r#"SELECT id, model, currency, input_price, output_price,
                              cache_read_input_price, cache_creation_input_price,
                              batch_input_price, batch_output_price,
                              priority_input_price, priority_output_price,
                              audio_input_price, source, region,
                              context_window, max_output_tokens,
                              supports_vision, supports_function_calling,
                              synced_at, created_at, updated_at
                       FROM prices WHERE region IS NOT DISTINCT FROM $1
                       ORDER BY model, currency LIMIT $2 OFFSET $3"#
                } else {
                    r#"SELECT id, model, currency, input_price, output_price,
                              cache_read_input_price, cache_creation_input_price,
                              batch_input_price, batch_output_price,
                              priority_input_price, priority_output_price,
                              audio_input_price, source, region,
                              context_window, max_output_tokens,
                              supports_vision, supports_function_calling,
                              synced_at, created_at, updated_at
                       FROM prices WHERE (region = ? OR (region IS NULL AND ? IS NULL))
                       ORDER BY model, currency LIMIT ? OFFSET ?"#
                };
                if is_postgres {
                    sqlx::query_as(sql)
                        .bind(reg)
                        .bind(limit)
                        .bind(offset)
                        .fetch_all(conn.pool())
                        .await?
                } else {
                    sqlx::query_as(sql)
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
                let sql = if is_postgres {
                    r#"SELECT id, model, currency, input_price, output_price,
                              cache_read_input_price, cache_creation_input_price,
                              batch_input_price, batch_output_price,
                              priority_input_price, priority_output_price,
                              audio_input_price, source, region,
                              context_window, max_output_tokens,
                              supports_vision, supports_function_calling,
                              synced_at, created_at, updated_at
                       FROM prices ORDER BY model, currency LIMIT $1 OFFSET $2"#
                } else {
                    r#"SELECT id, model, currency, input_price, output_price,
                              cache_read_input_price, cache_creation_input_price,
                              batch_input_price, batch_output_price,
                              priority_input_price, priority_output_price,
                              audio_input_price, source, region,
                              context_window, max_output_tokens,
                              supports_vision, supports_function_calling,
                              synced_at, created_at, updated_at
                       FROM prices ORDER BY model, currency LIMIT ? OFFSET ?"#
                };
                sqlx::query_as(sql)
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

        let sql = if is_postgres {
            r#"
            INSERT INTO prices (
                model, currency, input_price, output_price,
                cache_read_input_price, cache_creation_input_price,
                batch_input_price, batch_output_price,
                priority_input_price, priority_output_price,
                audio_input_price, source, region,
                context_window, max_output_tokens,
                supports_vision, supports_function_calling,
                synced_at, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
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
                source = EXCLUDED.source,
                context_window = EXCLUDED.context_window,
                max_output_tokens = EXCLUDED.max_output_tokens,
                supports_vision = EXCLUDED.supports_vision,
                supports_function_calling = EXCLUDED.supports_function_calling,
                synced_at = EXCLUDED.synced_at,
                updated_at = EXCLUDED.updated_at
            "#
        } else {
            r#"
            INSERT INTO prices (
                model, currency, input_price, output_price,
                cache_read_input_price, cache_creation_input_price,
                batch_input_price, batch_output_price,
                priority_input_price, priority_output_price,
                audio_input_price, source, region,
                context_window, max_output_tokens,
                supports_vision, supports_function_calling,
                synced_at, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(model, region) DO UPDATE SET
                currency = excluded.currency,
                input_price = excluded.input_price,
                output_price = excluded.output_price,
                cache_read_input_price = excluded.cache_read_input_price,
                cache_creation_input_price = excluded.cache_creation_input_price,
                batch_input_price = excluded.batch_input_price,
                batch_output_price = excluded.batch_output_price,
                priority_input_price = excluded.priority_input_price,
                priority_output_price = excluded.priority_output_price,
                audio_input_price = excluded.audio_input_price,
                source = excluded.source,
                context_window = excluded.context_window,
                max_output_tokens = excluded.max_output_tokens,
                supports_vision = excluded.supports_vision,
                supports_function_calling = excluded.supports_function_calling,
                synced_at = excluded.synced_at,
                updated_at = excluded.updated_at
            "#
        };

        sqlx::query(sql)
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
            .bind(&input.source)
            .bind(&input.region)
            .bind(input.context_window)
            .bind(input.max_output_tokens)
            .bind(input.supports_vision.map(|v| v as i32))
            .bind(input.supports_function_calling.map(|v| v as i32))
            .bind(now) // synced_at
            .bind(now) // created_at
            .bind(now) // updated_at
            .execute(conn.pool())
            .await?;

        Ok(())
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
                let sql = if is_postgres {
                    "DELETE FROM prices WHERE model = $1 AND currency = $2 AND region = $3"
                } else {
                    "DELETE FROM prices WHERE model = ? AND currency = ? AND region = ?"
                };
                sqlx::query(sql)
                    .bind(model)
                    .bind(currency)
                    .bind(r)
                    .execute(conn.pool())
                    .await?
            }
            None => {
                let sql = if is_postgres {
                    "DELETE FROM prices WHERE model = $1 AND currency = $2 AND region IS NULL"
                } else {
                    "DELETE FROM prices WHERE model = ? AND currency = ? AND region IS NULL"
                };
                sqlx::query(sql)
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
        let sql = match db.kind().as_str() {
            "postgres" => "DELETE FROM prices WHERE model = $1",
            _ => "DELETE FROM prices WHERE model = ?",
        };

        sqlx::query(sql).bind(model).execute(conn.pool()).await?;

        Ok(())
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
