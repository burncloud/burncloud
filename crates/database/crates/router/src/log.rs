//! Database operations for router logs and usage statistics
//!
//! This crate handles all database operations related to router logs,
//! usage statistics, and balance deductions.

use burncloud_database::{adapt_sql, ph, phs, Database, DatabaseError, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Router log entry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RouterLog {
    pub id: i64,
    pub request_id: String,
    pub user_id: Option<String>,
    pub path: String,
    pub upstream_id: Option<String>,
    pub status_code: i32,
    pub latency_ms: i64,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    #[sqlx(default)]
    /// Total cost in nanodollars (9 decimal precision)
    pub cost: i64,
    #[sqlx(default)]
    pub model: Option<String>,
    #[sqlx(default)]
    pub cache_read_tokens: i32,
    #[sqlx(default)]
    pub reasoning_tokens: i32,
    #[sqlx(default)]
    pub pricing_region: Option<String>,
    #[sqlx(default)]
    pub video_tokens: i32,
    // Per-type token counts (added in billing expansion)
    #[sqlx(default)]
    pub cache_write_tokens: i32,
    #[sqlx(default)]
    pub audio_input_tokens: i32,
    #[sqlx(default)]
    pub audio_output_tokens: i32,
    #[sqlx(default)]
    pub image_tokens: i32,
    #[sqlx(default)]
    pub embedding_tokens: i32,
    // Per-type cost breakdown in nanodollars
    #[sqlx(default)]
    pub input_cost: i64,
    #[sqlx(default)]
    pub output_cost: i64,
    #[sqlx(default)]
    pub cache_read_cost: i64,
    #[sqlx(default)]
    pub cache_write_cost: i64,
    #[sqlx(default)]
    pub audio_cost: i64,
    #[sqlx(default)]
    pub image_cost: i64,
    #[sqlx(default)]
    pub video_cost: i64,
    #[sqlx(default)]
    pub reasoning_cost: i64,
    #[sqlx(default)]
    pub embedding_cost: i64,
    // L6 Observability fields (migration 0011): which router layer made the
    // decision and what color was attached. Used by Grafana for affinity_hit /
    // shaper_reject / scorer_picked / failover_N reporting.
    #[sqlx(default)]
    pub layer_decision: Option<String>,
    #[sqlx(default)]
    pub traffic_color: Option<String>,
    pub created_at: Option<String>,
}

/// Usage statistics for a user over a time period
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStats {
    pub total_requests: i64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    /// Total cost in nanodollars
    pub total_cost_nano: i64,
}

/// Usage statistics grouped by model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageStats {
    pub model: String,
    pub requests: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cache_read_tokens: i64,
    pub reasoning_tokens: i64,
    /// Cost in nanodollars
    pub cost_nano: i64,
}

pub struct RouterLogModel;

impl RouterLogModel {
    /// Insert a new router log entry
    pub async fn insert(db: &Database, log: &RouterLog) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = format!(
            r#"
            INSERT INTO router_logs
            (request_id, user_id, path, upstream_id, status_code, latency_ms,
             prompt_tokens, completion_tokens, cost,
             model, cache_read_tokens, reasoning_tokens, pricing_region, video_tokens,
             cache_write_tokens, audio_input_tokens, audio_output_tokens, image_tokens, embedding_tokens,
             input_cost, output_cost, cache_read_cost, cache_write_cost,
             audio_cost, image_cost, video_cost, reasoning_cost, embedding_cost,
             layer_decision, traffic_color)
            VALUES ({})
            "#,
            phs(is_postgres, 30)
        );

        sqlx::query(&sql)
            .bind(&log.request_id)
            .bind(&log.user_id)
            .bind(&log.path)
            .bind(&log.upstream_id)
            .bind(log.status_code)
            .bind(log.latency_ms)
            .bind(log.prompt_tokens)
            .bind(log.completion_tokens)
            .bind(log.cost)
            .bind(&log.model)
            .bind(log.cache_read_tokens)
            .bind(log.reasoning_tokens)
            .bind(&log.pricing_region)
            .bind(log.video_tokens)
            .bind(log.cache_write_tokens)
            .bind(log.audio_input_tokens)
            .bind(log.audio_output_tokens)
            .bind(log.image_tokens)
            .bind(log.embedding_tokens)
            .bind(log.input_cost)
            .bind(log.output_cost)
            .bind(log.cache_read_cost)
            .bind(log.cache_write_cost)
            .bind(log.audio_cost)
            .bind(log.image_cost)
            .bind(log.video_cost)
            .bind(log.reasoning_cost)
            .bind(log.embedding_cost)
            .bind(&log.layer_decision)
            .bind(&log.traffic_color)
            .execute(conn.pool())
            .await?;

        // Update token used_quota
        if let Some(user_id) = &log.user_id {
            let total_tokens = log.prompt_tokens + log.completion_tokens;
            if total_tokens > 0 {
                let update_sql = adapt_sql(
                    is_postgres,
                    "UPDATE router_tokens SET used_quota = used_quota + ? WHERE user_id = ?",
                );
                sqlx::query(&update_sql)
                    .bind(total_tokens)
                    .bind(user_id)
                    .execute(conn.pool())
                    .await?;
            }
        }

        Ok(())
    }

    /// Get logs with pagination
    pub async fn get(db: &Database, limit: i32, offset: i32) -> Result<Vec<RouterLog>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = format!(
            "SELECT id, request_id, user_id, path, upstream_id, status_code, latency_ms, prompt_tokens, completion_tokens, cost, model, cache_read_tokens, reasoning_tokens, pricing_region, video_tokens, cache_write_tokens, audio_input_tokens, audio_output_tokens, image_tokens, embedding_tokens, input_cost, output_cost, cache_read_cost, cache_write_cost, audio_cost, image_cost, video_cost, reasoning_cost, embedding_cost, created_at FROM router_logs ORDER BY created_at DESC {}",
            adapt_sql(is_postgres, "LIMIT ? OFFSET ?")
        );
        let logs = sqlx::query_as::<_, RouterLog>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(conn.pool())
            .await?;
        Ok(logs)
    }

    /// Get logs with optional filtering by user_id, upstream_id (channel), and model
    pub async fn get_filtered(
        db: &Database,
        user_id: Option<&str>,
        upstream_id: Option<&str>,
        model: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<RouterLog>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let mut conditions: Vec<String> = Vec::new();
        let mut param_index = 1;

        if user_id.is_some() {
            conditions.push(format!("user_id = {}", ph(is_postgres, param_index)));
            param_index += 1;
        }
        if upstream_id.is_some() {
            conditions.push(format!("upstream_id = {}", ph(is_postgres, param_index)));
            param_index += 1;
        }
        if model.is_some() {
            conditions.push(format!("model = {}", ph(is_postgres, param_index)));
            param_index += 1;
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let limit_offset = format!(
            "LIMIT {} OFFSET {}",
            ph(is_postgres, param_index),
            ph(is_postgres, param_index + 1)
        );
        let sql = format!(
            "SELECT id, request_id, user_id, path, upstream_id, status_code, latency_ms, prompt_tokens, completion_tokens, cost, model, cache_read_tokens, reasoning_tokens, pricing_region, video_tokens, cache_write_tokens, audio_input_tokens, audio_output_tokens, image_tokens, embedding_tokens, input_cost, output_cost, cache_read_cost, cache_write_cost, audio_cost, image_cost, video_cost, reasoning_cost, embedding_cost, created_at FROM router_logs {} ORDER BY created_at DESC {}",
            where_clause, limit_offset
        );

        let mut query = sqlx::query_as::<_, RouterLog>(&sql);

        if let Some(uid) = user_id {
            query = query.bind(uid);
        }
        if let Some(upstream) = upstream_id {
            query = query.bind(upstream);
        }
        if let Some(m) = model {
            query = query.bind(m);
        }

        let logs = query
            .bind(limit)
            .bind(offset)
            .fetch_all(conn.pool())
            .await?;
        Ok(logs)
    }

    /// Get total usage by user
    pub async fn get_usage_by_user(db: &Database, user_id: &str) -> Result<(i64, i64)> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = adapt_sql(
            is_postgres,
            "SELECT SUM(prompt_tokens), SUM(completion_tokens) FROM router_logs WHERE user_id = ?",
        );
        let row: (Option<i64>, Option<i64>) = sqlx::query_as(&sql)
            .bind(user_id)
            .fetch_one(conn.pool())
            .await?;

        Ok((row.0.unwrap_or(0), row.1.unwrap_or(0)))
    }
}

/// Get aggregated usage statistics for a user over a time period
/// Period can be: "day", "week", "month"
pub async fn get_usage_stats(db: &Database, user_id: &str, period: &str) -> Result<UsageStats> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    // Calculate time threshold based on period
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| DatabaseError::Query(format!("Time error: {}", e)))?
        .as_secs() as i64;

    let threshold = match period {
        "day" => now - 24 * 60 * 60,
        "week" => now - 7 * 24 * 60 * 60,
        _ => now - 30 * 24 * 60 * 60, // Default to month for any other input
    };

    let time_filter = if is_postgres {
        format!("EXTRACT(EPOCH FROM created_at)::BIGINT >= {}", ph(is_postgres, 2))
    } else {
        "strftime('%s', created_at) >= CAST(? AS TEXT)".to_string()
    };

    let sql = format!(
        r#"
        SELECT
            COUNT(*) as total_requests,
            COALESCE(SUM(prompt_tokens), 0) as total_prompt_tokens,
            COALESCE(SUM(completion_tokens), 0) as total_completion_tokens,
            COALESCE(SUM(cost), 0) as total_cost
        FROM router_logs
        WHERE user_id = {} AND created_at IS NOT NULL AND {}
        "#,
        ph(is_postgres, 1),
        time_filter
    );

    let row: (Option<i64>, Option<i64>, Option<i64>, Option<i64>) = sqlx::query_as(&sql)
        .bind(user_id)
        .bind(threshold.to_string())
        .fetch_one(conn.pool())
        .await?;

    Ok(UsageStats {
        total_requests: row.0.unwrap_or(0),
        total_prompt_tokens: row.1.unwrap_or(0),
        total_completion_tokens: row.2.unwrap_or(0),
        total_cost_nano: row.3.unwrap_or(0),
    })
}

/// Get usage statistics grouped by model for a user over a time period
/// Period can be: "day", "week", "month"
pub async fn get_usage_stats_by_model(
    db: &Database,
    user_id: &str,
    period: &str,
) -> Result<Vec<ModelUsageStats>> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    // Calculate time threshold — same logic as get_usage_stats
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| DatabaseError::Query(format!("Time error: {}", e)))?
        .as_secs() as i64;

    let threshold = match period {
        "day" => now - 24 * 60 * 60,
        "week" => now - 7 * 24 * 60 * 60,
        _ => now - 30 * 24 * 60 * 60,
    };

    let time_filter = if is_postgres {
        format!("EXTRACT(EPOCH FROM created_at)::BIGINT >= {}", ph(is_postgres, 2))
    } else {
        "strftime('%s', created_at) >= CAST(? AS TEXT)".to_string()
    };

    let sql = format!(
        r#"
        SELECT
            COALESCE(model, 'Unknown') as model,
            COUNT(*) as requests,
            COALESCE(SUM(prompt_tokens), 0) as prompt_tokens,
            COALESCE(SUM(completion_tokens), 0) as completion_tokens,
            COALESCE(SUM(cache_read_tokens), 0) as cache_read_tokens,
            COALESCE(SUM(reasoning_tokens), 0) as reasoning_tokens,
            COALESCE(SUM(cost), 0) as cost
        FROM router_logs
        WHERE user_id = {} AND created_at IS NOT NULL AND {}
        GROUP BY model
        ORDER BY cost DESC
        "#,
        ph(is_postgres, 1),
        time_filter
    );

    let rows: Vec<(String, i64, i64, i64, i64, i64, i64)> = sqlx::query_as(&sql)
        .bind(user_id)
        .bind(threshold.to_string())
        .fetch_all(conn.pool())
        .await?;

    Ok(rows
        .into_iter()
        .map(
            |(
                model,
                requests,
                prompt_tokens,
                completion_tokens,
                cache_read_tokens,
                reasoning_tokens,
                cost_nano,
            )| ModelUsageStats {
                model,
                requests,
                prompt_tokens,
                completion_tokens,
                cache_read_tokens,
                reasoning_tokens,
                cost_nano,
            },
        )
        .collect())
}

/// Billing summary per model for reconciliation with upstream providers (e.g. Google).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingModelSummary {
    pub model: String,
    pub requests: i64,
    pub prompt_tokens: i64,
    pub cache_read_tokens: i64,
    pub completion_tokens: i64,
    pub reasoning_tokens: i64,
    /// Cost in USD (converted from nanodollars)
    pub cost_usd: f64,
}

/// Aggregate billing summary for a time period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingSummary {
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    /// Number of requests with NULL model (pre-migration data).
    pub pre_migration_requests: i64,
    pub models: Vec<BillingModelSummary>,
    pub total_cost_usd: f64,
}

/// Get aggregate billing summary grouped by model for internal reconciliation.
///
/// start/end are optional YYYY-MM-DD date strings.
/// Pre-migration rows (model IS NULL) are counted separately.
pub async fn get_billing_summary(
    db: &Database,
    start: Option<&str>,
    end: Option<&str>,
) -> Result<BillingSummary> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    // Build date filter clause
    // SQLite: created_at is TEXT (CURRENT_TIMESTAMP → "YYYY-MM-DD HH:MM:SS")
    //         use strftime to extract date portion for correct comparison
    // PostgreSQL: created_at is TIMESTAMP, cast to date
    let (date_filter, date_cast_start, date_cast_end) = match (start, end) {
        (Some(_), Some(_)) if is_postgres => (
            format!(
                "AND created_at::date >= {}::date AND created_at::date <= {}::date",
                ph(is_postgres, 1),
                ph(is_postgres, 2)
            ),
            true,
            true,
        ),
        (Some(_), None) if is_postgres => (
            format!("AND created_at::date >= {}::date", ph(is_postgres, 1)),
            true,
            false,
        ),
        (None, Some(_)) if is_postgres => (
            format!("AND created_at::date <= {}::date", ph(is_postgres, 1)),
            false,
            true,
        ),
        (Some(_), Some(_)) => (
            format!(
                "AND strftime('%Y-%m-%d', created_at) >= {} AND strftime('%Y-%m-%d', created_at) <= {}",
                ph(is_postgres, 1),
                ph(is_postgres, 2)
            ),
            true,
            true,
        ),
        (Some(_), None) => (
            format!(
                "AND strftime('%Y-%m-%d', created_at) >= {}",
                ph(is_postgres, 1)
            ),
            true,
            false,
        ),
        (None, Some(_)) => (
            format!(
                "AND strftime('%Y-%m-%d', created_at) <= {}",
                ph(is_postgres, 1)
            ),
            false,
            true,
        ),
        (None, None) => (String::new(), false, false),
    };

    // Count pre-migration (NULL model) rows
    let null_model_sql = format!(
        "SELECT COUNT(*) FROM router_logs WHERE model IS NULL {}",
        date_filter
    );
    let mut null_query = sqlx::query_scalar::<_, i64>(&null_model_sql);
    if date_cast_start {
        null_query = null_query.bind(start.unwrap_or(""));
    }
    if date_cast_end {
        null_query = null_query.bind(end.unwrap_or(""));
    }
    let pre_migration_requests: i64 = null_query.fetch_one(conn.pool()).await?;

    // Main query: GROUP BY model (exclude NULL model rows from model breakdown)
    let main_sql = format!(
        r#"
        SELECT
            model,
            COUNT(*) as requests,
            COALESCE(SUM(prompt_tokens), 0) as prompt_tokens,
            COALESCE(SUM(cache_read_tokens), 0) as cache_read_tokens,
            COALESCE(SUM(completion_tokens), 0) as completion_tokens,
            COALESCE(SUM(reasoning_tokens), 0) as reasoning_tokens,
            COALESCE(SUM(cost), 0) as cost_nano
        FROM router_logs
        WHERE model IS NOT NULL {}
        GROUP BY model
        ORDER BY cost_nano DESC
        "#,
        date_filter
    );

    let mut main_query = sqlx::query_as::<_, (String, i64, i64, i64, i64, i64, i64)>(&main_sql);
    if date_cast_start {
        main_query = main_query.bind(start.unwrap_or(""));
    }
    if date_cast_end {
        main_query = main_query.bind(end.unwrap_or(""));
    }
    let rows = main_query.fetch_all(conn.pool()).await?;

    let mut total_cost_nano: i64 = 0;
    let models: Vec<BillingModelSummary> = rows
        .into_iter()
        .map(
            |(
                model,
                requests,
                prompt_tokens,
                cache_read_tokens,
                completion_tokens,
                reasoning_tokens,
                cost_nano,
            )| {
                total_cost_nano = total_cost_nano.saturating_add(cost_nano);
                BillingModelSummary {
                    model,
                    requests,
                    prompt_tokens,
                    cache_read_tokens,
                    completion_tokens,
                    reasoning_tokens,
                    cost_usd: cost_nano as f64 / 1_000_000_000.0,
                }
            },
        )
        .collect();

    Ok(BillingSummary {
        period_start: start.map(|s| s.to_string()),
        period_end: end.map(|s| s.to_string()),
        pre_migration_requests,
        total_cost_usd: total_cost_nano as f64 / 1_000_000_000.0,
        models,
    })
}

/// Balance operations for dual-currency deduction
pub struct BalanceModel;

impl BalanceModel {
    /// Deduct from USD balance.
    /// Cost is in nanodollars (i64).
    /// Returns Ok(true) if deduction successful, Ok(false) if insufficient balance.
    pub async fn deduct_usd(db: &Database, user_id: &str, cost_nano: i64) -> Result<bool> {
        if cost_nano <= 0 {
            return Ok(true);
        }

        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        // Check current balance
        let balance_sql = adapt_sql(
            is_postgres,
            "SELECT COALESCE(balance_usd, 0) FROM user_accounts WHERE id = ?",
        );
        let balance: i64 = sqlx::query_scalar(&balance_sql)
            .bind(user_id)
            .fetch_optional(conn.pool())
            .await?
            .ok_or_else(|| DatabaseError::Query(format!("user account not found: {}", user_id)))?;

        if balance < cost_nano {
            return Ok(false);
        }

        // Deduct
        let deduct_sql = adapt_sql(
            is_postgres,
            "UPDATE user_accounts SET balance_usd = balance_usd - ? WHERE id = ? AND balance_usd >= ?",
        );
        let rows_affected = sqlx::query(&deduct_sql)
            .bind(cost_nano)
            .bind(user_id)
            .bind(cost_nano)
            .execute(conn.pool())
            .await?
            .rows_affected();

        Ok(rows_affected > 0)
    }

    /// Deduct from CNY balance.
    /// Cost is in nanodollars (i64).
    /// Returns Ok(true) if deduction successful, Ok(false) if insufficient balance.
    pub async fn deduct_cny(db: &Database, user_id: &str, cost_nano: i64) -> Result<bool> {
        if cost_nano <= 0 {
            return Ok(true);
        }

        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        // Check current balance
        let balance_sql = adapt_sql(
            is_postgres,
            "SELECT COALESCE(balance_cny, 0) FROM user_accounts WHERE id = ?",
        );
        let balance: i64 = sqlx::query_scalar(&balance_sql)
            .bind(user_id)
            .fetch_optional(conn.pool())
            .await?
            .flatten()
            .ok_or_else(|| DatabaseError::Query(format!("user account not found: {}", user_id)))?;

        if balance < cost_nano {
            return Ok(false);
        }

        // Deduct
        let deduct_sql = adapt_sql(
            is_postgres,
            "UPDATE user_accounts SET balance_cny = balance_cny - ? WHERE id = ? AND balance_cny >= ?",
        );
        let rows_affected = sqlx::query(&deduct_sql)
            .bind(cost_nano)
            .bind(user_id)
            .bind(cost_nano)
            .execute(conn.pool())
            .await?
            .rows_affected();

        Ok(rows_affected > 0)
    }

    /// Deduct cost from dual-currency wallet.
    /// Uses the primary currency first (based on cost_currency), then converts from secondary if needed.
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `user_id` - User ID to deduct from
    /// * `cost_nano` - Cost in nanodollars (i64)
    /// * `cost_currency` - Currency of the cost ("USD" or "CNY")
    /// * `exchange_rate_nano` - Exchange rate scaled by 10^9 (e.g., 7.24 CNY/USD = 7_240_000_000)
    ///
    /// # Returns
    /// Ok(true) if deduction successful, Ok(false) if insufficient balance across both currencies.
    pub async fn deduct_dual_currency(
        db: &Database,
        user_id: &str,
        cost_nano: i64,
        cost_currency: &str,
        exchange_rate_nano: i64,
    ) -> Result<bool> {
        if cost_nano <= 0 {
            return Ok(true);
        }

        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        // Get current balances
        let balances_sql = adapt_sql(
            is_postgres,
            "SELECT COALESCE(balance_usd, 0), COALESCE(balance_cny, 0) FROM user_accounts WHERE id = ?",
        );
        let balances: Option<(i64, i64)> = sqlx::query_as(&balances_sql)
            .bind(user_id)
            .fetch_optional(conn.pool())
            .await?;

        let (balance_usd, balance_cny) = balances
            .ok_or_else(|| DatabaseError::Query(format!("user account not found: {}", user_id)))?;

        if cost_currency == "CNY" {
            // CNY model: prioritize CNY balance
            if balance_cny >= cost_nano {
                // Sufficient CNY balance
                return Self::deduct_cny(db, user_id, cost_nano).await;
            }

            // Need to convert USD to CNY
            let required_cny = cost_nano - balance_cny;
            let required_usd: i128 =
                (required_cny as i128 * 1_000_000_000) / exchange_rate_nano as i128;

            if required_usd > balance_usd as i128 {
                return Ok(false);
            }

            // Deduct from both currencies atomically
            let mut tx = conn.pool().begin().await?;

            let clear_cny_sql = adapt_sql(
                is_postgres,
                "UPDATE user_accounts SET balance_cny = 0 WHERE id = ?",
            );

            if balance_cny > 0 {
                sqlx::query(&clear_cny_sql)
                    .bind(user_id)
                    .execute(&mut *tx)
                    .await?;
            }

            let usd_to_deduct = required_usd as i64;
            let deduct_usd_sql = adapt_sql(
                is_postgres,
                "UPDATE user_accounts SET balance_usd = balance_usd - ? WHERE id = ? AND balance_usd >= ?",
            );
            sqlx::query(&deduct_usd_sql)
                .bind(usd_to_deduct)
                .bind(user_id)
                .bind(usd_to_deduct)
                .execute(&mut *tx)
                .await?;

            tx.commit().await?;
            Ok(true)
        } else {
            // USD model (default): prioritize USD balance
            if balance_usd >= cost_nano {
                return Self::deduct_usd(db, user_id, cost_nano).await;
            }

            // Need to convert CNY to USD
            let required_usd = cost_nano - balance_usd;
            let required_cny: i128 =
                (required_usd as i128 * exchange_rate_nano as i128) / 1_000_000_000;

            if required_cny > balance_cny as i128 {
                return Ok(false);
            }

            // Deduct from both currencies atomically
            let mut tx = conn.pool().begin().await?;

            let clear_usd_sql = adapt_sql(
                is_postgres,
                "UPDATE user_accounts SET balance_usd = 0 WHERE id = ?",
            );

            if balance_usd > 0 {
                sqlx::query(&clear_usd_sql)
                    .bind(user_id)
                    .execute(&mut *tx)
                    .await?;
            }

            let cny_to_deduct = required_cny as i64;
            let deduct_cny_sql = adapt_sql(
                is_postgres,
                "UPDATE user_accounts SET balance_cny = balance_cny - ? WHERE id = ? AND balance_cny >= ?",
            );
            sqlx::query(&deduct_cny_sql)
                .bind(cny_to_deduct)
                .bind(user_id)
                .bind(cny_to_deduct)
                .execute(&mut *tx)
                .await?;

            tx.commit().await?;
            Ok(true)
        }
    }
}
