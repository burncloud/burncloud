//! Identity-owned wallet settlement. The balance columns in `user_accounts`
//! have exactly one writer module; Traffic invokes this public capability.
//!
//! Preserve the two historical call paths, including their distinct
//! missing-user semantics. Unifying them is a separate behavioral decision.

use burncloud_database::{adapt_sql, Database, DatabaseError, Result};

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

impl BalanceModel {
    /// Legacy RouterDatabase debit semantics (a missing account behaves like
    /// a zero-balance account). Unlike the service path, this historically
    /// returns Ok(false) for a positive cost on a missing user.
    pub async fn deduct_dual_currency_router_legacy(
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
        let balances_sql = adapt_sql(is_postgres, "SELECT COALESCE(balance_usd, 0), COALESCE(balance_cny, 0) FROM user_accounts WHERE id = ?");
        let balances: Option<(i64, i64)> = sqlx::query_as(&balances_sql)
            .bind(user_id)
            .fetch_optional(conn.pool())
            .await?;

        let (balance_usd, balance_cny) = balances.unwrap_or((0, 0));

        if cost_currency == "CNY" {
            // CNY model: prioritize CNY balance
            if balance_cny >= cost_nano {
                // Sufficient CNY balance
                return Self::deduct_cny(db, user_id, cost_nano).await;
            }

            // Need to convert USD to CNY
            // Required CNY = cost_nano - balance_cny
            // Required USD in nanodollars = required_cny * 10^9 / exchange_rate_nano
            // Using i128 for intermediate calculation to avoid overflow
            let required_cny = cost_nano - balance_cny;
            let required_usd: i128 =
                (required_cny as i128 * 1_000_000_000) / exchange_rate_nano as i128;

            if required_usd > balance_usd as i128 {
                // Insufficient total balance
                return Ok(false);
            }

            // Deduct from both currencies atomically
            let mut tx = conn.pool().begin().await?;

            let clear_cny_sql = adapt_sql(
                is_postgres,
                "UPDATE user_accounts SET balance_cny = 0 WHERE id = ?",
            );

            // Deduct remaining CNY
            if balance_cny > 0 {
                sqlx::query(&clear_cny_sql)
                    .bind(user_id)
                    .execute(&mut *tx)
                    .await?;
            }

            // Deduct required USD (already integer, no need to round)
            let usd_to_deduct = required_usd as i64;
            let deduct_usd_sql = adapt_sql(is_postgres, "UPDATE user_accounts SET balance_usd = balance_usd - ? WHERE id = ? AND balance_usd >= ?");
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
                // Sufficient USD balance
                return Self::deduct_usd(db, user_id, cost_nano).await;
            }

            // Need to convert CNY to USD
            // Required USD = cost_nano - balance_usd
            // Required CNY in nanodollars = required_usd * exchange_rate_nano / 10^9
            // Using i128 for intermediate calculation to avoid overflow
            let required_usd = cost_nano - balance_usd;
            let required_cny: i128 =
                (required_usd as i128 * exchange_rate_nano as i128) / 1_000_000_000;

            if required_cny > balance_cny as i128 {
                // Insufficient total balance
                return Ok(false);
            }

            // Deduct from both currencies atomically
            let mut tx = conn.pool().begin().await?;

            let clear_usd_sql = adapt_sql(
                is_postgres,
                "UPDATE user_accounts SET balance_usd = 0 WHERE id = ?",
            );

            // Deduct remaining USD
            if balance_usd > 0 {
                sqlx::query(&clear_usd_sql)
                    .bind(user_id)
                    .execute(&mut *tx)
                    .await?;
            }

            // Deduct required CNY (already integer, no need to round)
            let cny_to_deduct = required_cny as i64;
            let deduct_cny_sql = adapt_sql(is_postgres, "UPDATE user_accounts SET balance_cny = balance_cny - ? WHERE id = ? AND balance_cny >= ?");
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
