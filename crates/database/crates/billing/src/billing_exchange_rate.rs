use burncloud_common::types::BillingExchangeRate;
use burncloud_database::{adapt_sql, Database, Result};

pub struct BillingExchangeRateModel;

impl BillingExchangeRateModel {
    /// Fetch the most recent exchange rate for a currency pair.
    pub async fn get_rate(
        db: &Database,
        from_currency: &str,
        to_currency: &str,
    ) -> Result<Option<BillingExchangeRate>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = adapt_sql(
            is_postgres,
            "SELECT id, from_currency, to_currency, rate, updated_at \
             FROM billing_exchange_rates \
             WHERE from_currency = ? AND to_currency = ? \
             ORDER BY updated_at DESC NULLS LAST LIMIT 1",
        );
        let rate: Option<BillingExchangeRate> = sqlx::query_as(&sql)
            .bind(from_currency)
            .bind(to_currency)
            .fetch_optional(conn.pool())
            .await?;
        Ok(rate)
    }

    /// Upsert a new exchange rate (by from/to pair).
    pub async fn upsert(
        db: &Database,
        from_currency: &str,
        to_currency: &str,
        rate: i64,
        updated_at: Option<i64>,
    ) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = if is_postgres {
            "INSERT INTO billing_exchange_rates (from_currency, to_currency, rate, updated_at) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (from_currency, to_currency) DO UPDATE \
             SET rate = EXCLUDED.rate, updated_at = EXCLUDED.updated_at"
        } else {
            "INSERT INTO billing_exchange_rates (from_currency, to_currency, rate, updated_at) \
             VALUES (?, ?, ?, ?) \
             ON CONFLICT (from_currency, to_currency) DO UPDATE \
             SET rate = excluded.rate, updated_at = excluded.updated_at"
        };
        sqlx::query(sql)
            .bind(from_currency)
            .bind(to_currency)
            .bind(rate)
            .bind(updated_at)
            .execute(conn.pool())
            .await?;
        Ok(())
    }
}
