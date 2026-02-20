//! Exchange Rate Service Module
//!
//! This module provides currency conversion functionality with caching support.
//! Supports USD, CNY, and EUR currencies.

use std::str::FromStr;
use std::sync::Arc;

use burncloud_common::Currency;
use burncloud_database::{sqlx, Database};
use chrono::{DateTime, Utc};
use dashmap::DashMap;

/// Exchange rate entry with timestamp
#[derive(Debug, Clone)]
pub struct CachedRate {
    pub rate: f64,
    pub updated_at: DateTime<Utc>,
}

/// Service for managing exchange rates and currency conversion
pub struct ExchangeRateService {
    db: Arc<Database>,
    /// In-memory cache for exchange rates: (from_currency, to_currency) -> rate
    rates: DashMap<(Currency, Currency), CachedRate>,
}

impl ExchangeRateService {
    /// Create a new ExchangeRateService
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            rates: DashMap::new(),
        }
    }

    /// Convert an amount from one currency to another
    pub fn convert(&self, amount: f64, from: Currency, to: Currency) -> f64 {
        if from == to {
            return amount;
        }

        if let Some(rate) = self.get_rate(from, to) {
            amount * rate
        } else {
            // Fallback: try reverse rate
            if let Some(reverse_rate) = self.get_rate(to, from) {
                if reverse_rate > 0.0 {
                    amount / reverse_rate
                } else {
                    amount
                }
            } else {
                // No rate available, return original amount
                tracing::warn!(
                    "No exchange rate found for {} -> {}, returning original amount",
                    from, to
                );
                amount
            }
        }
    }

    /// Get the exchange rate from one currency to another
    pub fn get_rate(&self, from: Currency, to: Currency) -> Option<f64> {
        if from == to {
            return Some(1.0);
        }

        self.rates.get(&(from, to)).map(|r| r.rate)
    }

    /// Set an exchange rate in the cache
    pub fn set_rate(&self, from: Currency, to: Currency, rate: f64) {
        self.rates.insert(
            (from, to),
            CachedRate {
                rate,
                updated_at: Utc::now(),
            },
        );
    }

    /// Load exchange rates from the database into cache
    pub async fn load_rates_from_db(&self) -> anyhow::Result<usize> {
        let conn = self.db.get_connection()?;
        let sql = "SELECT from_currency, to_currency, rate, updated_at FROM exchange_rates";

        let rows = sqlx::query_as::<_, (String, String, f64, Option<i64>)>(sql)
            .fetch_all(conn.pool())
            .await?;

        let mut count = 0;
        for (from, to, rate, updated_at) in rows {
            if let (Ok(from_currency), Ok(to_currency)) =
                (Currency::from_str(&from), Currency::from_str(&to))
            {
                let updated = updated_at
                    .map(|ts| DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now))
                    .unwrap_or_else(Utc::now);

                self.rates.insert(
                    (from_currency, to_currency),
                    CachedRate {
                        rate,
                        updated_at: updated,
                    },
                );
                count += 1;
            }
        }

        tracing::info!("Loaded {} exchange rates from database", count);
        Ok(count)
    }

    /// Save an exchange rate to the database
    pub async fn save_rate_to_db(
        &self,
        from: Currency,
        to: Currency,
        rate: f64,
    ) -> anyhow::Result<()> {
        let conn = self.db.get_connection()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let sql = match self.db.kind().as_str() {
            "postgres" => r#"
                INSERT INTO exchange_rates (from_currency, to_currency, rate, updated_at)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT(from_currency, to_currency) DO UPDATE SET
                    rate = EXCLUDED.rate,
                    updated_at = EXCLUDED.updated_at
            "#,
            _ => r#"
                INSERT INTO exchange_rates (from_currency, to_currency, rate, updated_at)
                VALUES (?, ?, ?, ?)
                ON CONFLICT(from_currency, to_currency) DO UPDATE SET
                    rate = excluded.rate,
                    updated_at = excluded.updated_at
            "#,
        };

        sqlx::query(sql)
            .bind(from.code())
            .bind(to.code())
            .bind(rate)
            .bind(now)
            .execute(conn.pool())
            .await?;

        // Update cache
        self.set_rate(from, to, rate);

        tracing::info!("Saved exchange rate {} -> {}: {}", from, to, rate);
        Ok(())
    }

    /// Get all cached exchange rates
    pub fn list_rates(&self) -> Vec<(Currency, Currency, f64, DateTime<Utc>)> {
        self.rates
            .iter()
            .map(|entry| {
                let key = entry.key();
                (key.0, key.1, entry.rate, entry.updated_at)
            })
            .collect()
    }

    /// Clear the exchange rate cache
    pub fn clear_cache(&self) {
        self.rates.clear();
    }

    /// Get the last update time for a specific rate
    pub fn get_last_updated(&self, from: Currency, to: Currency) -> Option<DateTime<Utc>> {
        self.rates.get(&(from, to)).map(|r| r.updated_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Create a mock database for testing
    /// Since ExchangeRateService tests only use in-memory cache, we can use a minimal mock
    fn create_test_service() -> ExchangeRateService {
        // For unit tests that don't need DB persistence, we can create a service
        // with a database that will fail on DB operations but work for cache operations
        // Since Database::new is async, we use a simpler approach: tests that need DB
        // should be integration tests. For unit tests, we only test cache operations.
        use burncloud_database::Database;
        use std::sync::Arc;

        // Use tokio runtime to create database with unique path
        let rt = tokio::runtime::Runtime::new().unwrap();
        let db = rt.block_on(async {
            // Generate unique database path to avoid conflicts between tests
            let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
            let db_path = format!("/tmp/burncloud_test_exchange_rate_{}_{}.db",
                std::process::id(), test_id);

            // Remove existing test db if exists
            let _ = std::fs::remove_file(&db_path);

            // Set environment variable for database path
            std::env::set_var("BURNCLOUD_DATABASE_URL", format!("sqlite://{}?mode=rwc", db_path));

            let db = Database::new().await.unwrap();
            db
        });
        ExchangeRateService::new(Arc::new(db))
    }

    #[test]
    fn test_convert_same_currency() {
        let service = create_test_service();

        let amount = service.convert(100.0, Currency::USD, Currency::USD);
        assert!((amount - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_set_and_get_rate() {
        let service = create_test_service();

        service.set_rate(Currency::USD, Currency::CNY, 7.2);

        let rate = service.get_rate(Currency::USD, Currency::CNY);
        assert_eq!(rate, Some(7.2));

        let rate = service.get_rate(Currency::CNY, Currency::USD);
        assert_eq!(rate, None);
    }

    #[test]
    fn test_convert_with_rate() {
        let service = create_test_service();

        service.set_rate(Currency::USD, Currency::CNY, 7.2);

        let amount = service.convert(100.0, Currency::USD, Currency::CNY);
        assert!((amount - 720.0).abs() < 0.001);

        // Reverse conversion using reverse rate
        let amount = service.convert(720.0, Currency::CNY, Currency::USD);
        assert!((amount - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_convert_missing_rate() {
        let service = create_test_service();

        // No rate set, should return original amount
        let amount = service.convert(100.0, Currency::USD, Currency::EUR);
        assert!((amount - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_list_rates() {
        let service = create_test_service();

        service.set_rate(Currency::USD, Currency::CNY, 7.2);
        service.set_rate(Currency::EUR, Currency::USD, 1.08);

        let rates = service.list_rates();
        assert_eq!(rates.len(), 2);
    }

    #[test]
    fn test_clear_cache() {
        let service = create_test_service();

        service.set_rate(Currency::USD, Currency::CNY, 7.2);
        assert_eq!(service.get_rate(Currency::USD, Currency::CNY), Some(7.2));

        service.clear_cache();
        assert_eq!(service.get_rate(Currency::USD, Currency::CNY), None);
    }
}
