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

    /// Deduct from the primary currency, then the secondary currency if required.
    /// A missing user is an error on the service-facing path.
    pub async fn deduct_dual_currency(
        db: &Database,
        user_id: &str,
        cost_nano: i64,
        cost_currency: &str,
        exchange_rate_nano: i64,
    ) -> Result<bool> {
        Self::deduct_dual_atomic(
            db,
            user_id,
            cost_nano,
            cost_currency,
            exchange_rate_nano,
            false,
        )
        .await
    }

    /// RouterDatabase historically returns Ok(false) for a missing user.
    /// Keep that distinct error contract while using the same atomic settlement.
    pub async fn deduct_dual_currency_router_legacy(
        db: &Database,
        user_id: &str,
        cost_nano: i64,
        cost_currency: &str,
        exchange_rate_nano: i64,
    ) -> Result<bool> {
        Self::deduct_dual_atomic(
            db,
            user_id,
            cost_nano,
            cost_currency,
            exchange_rate_nano,
            true,
        )
        .await
    }

    /// Hold one wallet transaction for the balance read and every debit.
    /// PostgreSQL locks the wallet row so concurrent spenders cannot both
    /// budget from the same snapshot. SQLite detects conflicting writers;
    /// neither driver is permitted to commit a partial successful charge.
    async fn deduct_dual_atomic(
        db: &Database,
        user_id: &str,
        cost_nano: i64,
        cost_currency: &str,
        exchange_rate_nano: i64,
        missing_as_insufficient: bool,
    ) -> Result<bool> {
        // Historical semantics: zero and negative charges do not mutate data.
        if cost_nano <= 0 {
            return Ok(true);
        }

        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let mut tx = conn.pool().begin().await?;
        let select_sql = if is_postgres {
            adapt_sql(
                true,
                "SELECT COALESCE(balance_usd, 0), COALESCE(balance_cny, 0) FROM user_accounts WHERE id = ? FOR UPDATE",
            )
        } else {
            adapt_sql(
                false,
                "SELECT COALESCE(balance_usd, 0), COALESCE(balance_cny, 0) FROM user_accounts WHERE id = ?",
            )
        };
        let balances: Option<(i64, i64)> = sqlx::query_as(&select_sql)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;

        let (balance_usd, balance_cny) = match balances {
            Some(balances) => balances,
            None if missing_as_insufficient => return Ok(false),
            None => {
                return Err(DatabaseError::Query(format!(
                    "user account not found: {user_id}"
                )));
            }
        };
        if balance_usd < 0 || balance_cny < 0 {
            return Err(DatabaseError::InvalidData {
                message: "wallet cannot be charged with a negative currency balance".into(),
            });
        }

        let cny_primary = cost_currency == "CNY";
        let primary_balance = if cny_primary {
            balance_cny
        } else {
            balance_usd
        };
        if primary_balance >= cost_nano {
            // The single-currency path also stays inside this transaction.
            let debit_sql = if cny_primary {
                "UPDATE user_accounts SET balance_cny = balance_cny - ? WHERE id = ? AND balance_cny >= ?"
            } else {
                "UPDATE user_accounts SET balance_usd = balance_usd - ? WHERE id = ? AND balance_usd >= ?"
            };
            let affected = sqlx::query(&adapt_sql(is_postgres, debit_sql))
                .bind(cost_nano)
                .bind(user_id)
                .bind(cost_nano)
                .execute(&mut *tx)
                .await?
                .rows_affected();
            if affected != 1 {
                return Ok(false);
            }
            tx.commit().await?;
            return Ok(true);
        }

        // A conversion rate is only needed when primary funds are insufficient.
        if exchange_rate_nano <= 0 {
            return Err(DatabaseError::InvalidData {
                message: "exchange rate must be positive for cross-currency debit".into(),
            });
        }

        let missing_primary = i128::from(cost_nano) - i128::from(primary_balance);
        let secondary_required = if cny_primary {
            (missing_primary * 1_000_000_000) / i128::from(exchange_rate_nano)
        } else {
            (missing_primary * i128::from(exchange_rate_nano)) / 1_000_000_000
        };
        let secondary_available = if cny_primary {
            balance_usd
        } else {
            balance_cny
        };
        if secondary_required > i128::from(secondary_available)
            || secondary_required > i128::from(i64::MAX)
        {
            return Ok(false);
        }
        // Positive inputs and a positive exchange rate guarantee a
        // nonnegative result. Division intentionally retains the original
        // integer truncation rule in nanodollars.
        let secondary_to_debit = secondary_required as i64;

        if primary_balance > 0 {
            let clear_sql = if cny_primary {
                "UPDATE user_accounts SET balance_cny = 0 WHERE id = ? AND balance_cny = ?"
            } else {
                "UPDATE user_accounts SET balance_usd = 0 WHERE id = ? AND balance_usd = ?"
            };
            let cleared = sqlx::query(&adapt_sql(is_postgres, clear_sql))
                .bind(user_id)
                .bind(primary_balance)
                .execute(&mut *tx)
                .await?
                .rows_affected();
            if cleared != 1 {
                return Ok(false);
            }
        }

        let secondary_sql = if cny_primary {
            "UPDATE user_accounts SET balance_usd = balance_usd - ? WHERE id = ? AND balance_usd >= ?"
        } else {
            "UPDATE user_accounts SET balance_cny = balance_cny - ? WHERE id = ? AND balance_cny >= ?"
        };
        let debited = sqlx::query(&adapt_sql(is_postgres, secondary_sql))
            .bind(secondary_to_debit)
            .bind(user_id)
            .bind(secondary_to_debit)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        if debited != 1 {
            // Dropping the transaction rolls back the primary clear too.
            return Ok(false);
        }
        tx.commit().await?;
        Ok(true)
    }
}
