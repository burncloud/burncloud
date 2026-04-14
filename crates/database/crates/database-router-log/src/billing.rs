//! Billing reconciliation and balance deduction operations.
//!
//! This module is responsible for:
//! - Generating aggregate billing summaries grouped by model
//! - Dual-currency balance deductions (USD / CNY) from the `users` table

use burncloud_database::{adapt_sql, Database, Result};
use serde::{Deserialize, Serialize};

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
/// `start` / `end` are optional YYYY-MM-DD date strings.
/// Pre-migration rows (model IS NULL) are counted separately.
pub async fn get_billing_summary(
    db: &Database,
    start: Option<&str>,
    end: Option<&str>,
) -> Result<BillingSummary> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    // Build date filter clause.
    // SQLite: created_at is TEXT (CURRENT_TIMESTAMP → "YYYY-MM-DD HH:MM:SS")
    //         use strftime to extract date portion for correct comparison
    // PostgreSQL: created_at is TIMESTAMP, cast to date
    let (date_filter, date_cast_start, date_cast_end) = match (start, end) {
        (Some(_), Some(_)) if is_postgres => (
            "AND created_at::date >= $1::date AND created_at::date <= $2::date",
            true,
            true,
        ),
        (Some(_), None) if is_postgres => ("AND created_at::date >= $1::date", true, false),
        (None, Some(_)) if is_postgres => ("AND created_at::date <= $1::date", false, true),
        (Some(_), Some(_)) => (
            "AND strftime('%Y-%m-%d', created_at) >= ? AND strftime('%Y-%m-%d', created_at) <= ?",
            true,
            true,
        ),
        (Some(_), None) => ("AND strftime('%Y-%m-%d', created_at) >= ?", true, false),
        (None, Some(_)) => ("AND strftime('%Y-%m-%d', created_at) <= ?", false, true),
        (None, None) => ("", false, false),
    };

    // Count pre-migration (NULL model) rows.
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
    let pre_migration_requests: i64 = null_query.fetch_one(conn.pool()).await.unwrap_or(0);

    // Main query: GROUP BY model (exclude NULL model rows from model breakdown).
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

    let mut main_query =
        sqlx::query_as::<_, (String, i64, i64, i64, i64, i64, i64)>(&main_sql);
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

/// Balance operations for dual-currency deduction from the `users` table.
pub struct BalanceModel;

impl BalanceModel {
    /// Deduct from USD balance.
    ///
    /// Cost is in nanodollars (i64).
    /// Returns `Ok(true)` if deduction successful, `Ok(false)` if insufficient balance.
    pub async fn deduct_usd(db: &Database, user_id: &str, cost_nano: i64) -> Result<bool> {
        if cost_nano <= 0 {
            return Ok(true);
        }

        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let balance_sql = adapt_sql(is_postgres, "SELECT COALESCE(balance_usd, 0) FROM users WHERE id = ?");
        let balance: i64 = sqlx::query_scalar(&balance_sql)
            .bind(user_id)
            .fetch_one(conn.pool())
            .await
            .unwrap_or(0);

        if balance < cost_nano {
            return Ok(false);
        }

        let deduct_sql = adapt_sql(is_postgres, "UPDATE users SET balance_usd = balance_usd - ? WHERE id = ? AND balance_usd >= ?");
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
    ///
    /// Cost is in nanodollars (i64).
    /// Returns `Ok(true)` if deduction successful, `Ok(false)` if insufficient balance.
    pub async fn deduct_cny(db: &Database, user_id: &str, cost_nano: i64) -> Result<bool> {
        if cost_nano <= 0 {
            return Ok(true);
        }

        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let balance_sql = adapt_sql(is_postgres, "SELECT COALESCE(balance_cny, 0) FROM users WHERE id = ?");
        let balance: i64 = sqlx::query_scalar(&balance_sql)
            .bind(user_id)
            .fetch_one(conn.pool())
            .await
            .unwrap_or(0);

        if balance < cost_nano {
            return Ok(false);
        }

        let deduct_sql = adapt_sql(is_postgres, "UPDATE users SET balance_cny = balance_cny - ? WHERE id = ? AND balance_cny >= ?");
        let rows_affected = sqlx::query(&deduct_sql)
            .bind(cost_nano)
            .bind(user_id)
            .bind(cost_nano)
            .execute(conn.pool())
            .await?
            .rows_affected();

        Ok(rows_affected > 0)
    }

    /// Deduct cost from a dual-currency wallet.
    ///
    /// Uses the primary currency first (based on `cost_currency`), then converts
    /// from the secondary currency if needed.
    ///
    /// # Arguments
    /// * `cost_nano`          — Cost in nanodollars
    /// * `cost_currency`      — `"USD"` or `"CNY"`
    /// * `exchange_rate_nano` — CNY/USD exchange rate scaled by 10^9
    ///                          (e.g. 7.24 CNY/USD → 7_240_000_000)
    ///
    /// Returns `Ok(true)` if deduction successful, `Ok(false)` if total balance is insufficient.
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

        let balances_sql = adapt_sql(is_postgres, "SELECT COALESCE(balance_usd, 0), COALESCE(balance_cny, 0) FROM users WHERE id = ?");
        let balances: Option<(i64, i64)> = sqlx::query_as(&balances_sql)
            .bind(user_id)
            .fetch_optional(conn.pool())
            .await?;

        let (balance_usd, balance_cny) = balances.unwrap_or((0, 0));

        if cost_currency == "CNY" {
            if balance_cny >= cost_nano {
                return Self::deduct_cny(db, user_id, cost_nano).await;
            }

            let required_cny = cost_nano - balance_cny;
            let required_usd: i128 =
                (required_cny as i128 * 1_000_000_000) / exchange_rate_nano as i128;

            if required_usd > balance_usd as i128 {
                return Ok(false);
            }

            let mut tx = conn.pool().begin().await?;

            let clear_cny_sql = adapt_sql(is_postgres, "UPDATE users SET balance_cny = 0 WHERE id = ?");

            if balance_cny > 0 {
                sqlx::query(&clear_cny_sql)
                    .bind(user_id)
                    .execute(&mut *tx)
                    .await?;
            }

            let usd_to_deduct = required_usd as i64;
            let deduct_usd_sql = adapt_sql(is_postgres, "UPDATE users SET balance_usd = balance_usd - ? WHERE id = ? AND balance_usd >= ?");
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

            let required_usd = cost_nano - balance_usd;
            let required_cny: i128 =
                (required_usd as i128 * exchange_rate_nano as i128) / 1_000_000_000;

            if required_cny > balance_cny as i128 {
                return Ok(false);
            }

            let mut tx = conn.pool().begin().await?;

            let clear_usd_sql = adapt_sql(is_postgres, "UPDATE users SET balance_usd = 0 WHERE id = ?");

            if balance_usd > 0 {
                sqlx::query(&clear_usd_sql)
                    .bind(user_id)
                    .execute(&mut *tx)
                    .await?;
            }

            let cny_to_deduct = required_cny as i64;
            let deduct_cny_sql = adapt_sql(is_postgres, "UPDATE users SET balance_cny = balance_cny - ? WHERE id = ? AND balance_cny >= ?");
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
