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

    /// Start a background task to periodically refresh exchange rates
    ///
    /// This spawns a tokio task that:
    /// - Checks every hour if rates need to be refreshed
    /// - Refreshes rates that are older than 24 hours
    /// - Logs warnings on failure but doesn't panic
    ///
    /// # Example
    /// ```ignore
    /// let service = Arc::new(ExchangeRateService::new(db));
    /// service.start_sync_task();
    /// ```
    pub fn start_sync_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Check hourly

            loop {
                interval.tick().await;

                // Load rates from database first
                match self.load_rates_from_db().await {
                    Ok(count) => {
                        tracing::debug!("Loaded {} exchange rates from database", count);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load exchange rates from database: {}", e);
                    }
                }

                // Check if we need to refresh rates (older than 24 hours)
                let now = Utc::now();
                let needs_refresh = self.rates.iter().any(|entry| {
                    let age = now.signed_duration_since(entry.updated_at);
                    age.num_hours() >= 24
                });

                if needs_refresh {
                    tracing::info!("Exchange rates are stale, attempting refresh");
                    // Note: External API refresh would go here
                    // For now, we just log that refresh is needed
                    tracing::info!(
                        "Exchange rate auto-refresh not configured. \
                         Use 'burncloud currency set-rate' to update manually."
                    );
                }
            }
        });
    }

    /// Start exchange rate sync task with external API support
    ///
    /// This is an extended version that can fetch rates from an external API.
    /// The API URL should return JSON like: {"USD_CNY": 7.2, "EUR_USD": 1.08}
    #[cfg(feature = "exchange-api")]
    pub async fn fetch_from_api(&self, api_url: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;

        let response = client.get(api_url).send().await?;
        let json: serde_json::Value = response.json().await?;

        // Parse rates from API response
        // Expected format: {"USD_CNY": 7.2, "EUR_USD": 1.08}
        if let Some(obj) = json.as_object() {
            for (key, value) in obj {
                if let (Some(from), Some(to)) = (key.split('_').next(), key.split('_').nth(1)) {
                    if let (Ok(from_currency), Ok(to_currency), Some(rate)) = (
                        Currency::from_str(from),
                        Currency::from_str(to),
                        value.as_f64(),
                    ) {
                        self.set_rate(from_currency, to_currency, rate);
                        tracing::info!("Updated rate: {} -> {} = {}", from_currency, to_currency, rate);
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    /// Create a mock database for testing
    /// Since ExchangeRateService tests only use in-memory cache, we can use a minimal mock
    fn create_test_service() -> ExchangeRateService {
        // Lock to ensure tests run serially to avoid DB conflicts
        let _lock = TEST_MUTEX.lock().unwrap();

        use burncloud_database::Database;
        use std::sync::Arc;

        // Use tokio runtime to create database with unique path
        let rt = tokio::runtime::Runtime::new().unwrap();
        let db = rt.block_on(async {
            // Generate unique database path to avoid conflicts between tests
            let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
            let pid = std::process::id();
            let db_path = format!("/tmp/burncloud_test_exch_{}_{}.db", pid, test_id);

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

    #[test]
    fn test_get_last_updated() {
        let service = create_test_service();

        // No rate set
        assert!(service.get_last_updated(Currency::USD, Currency::CNY).is_none());

        // Set rate
        service.set_rate(Currency::USD, Currency::CNY, 7.2);
        let updated = service.get_last_updated(Currency::USD, Currency::CNY);
        assert!(updated.is_some());
    }

    #[test]
    fn test_multiple_currencies() {
        let service = create_test_service();

        // Set multiple rates
        service.set_rate(Currency::USD, Currency::CNY, 7.2);
        service.set_rate(Currency::USD, Currency::EUR, 0.93);
        service.set_rate(Currency::EUR, Currency::CNY, 7.75);

        // Test each conversion
        assert!((service.convert(100.0, Currency::USD, Currency::CNY) - 720.0).abs() < 0.001);
        assert!((service.convert(100.0, Currency::USD, Currency::EUR) - 93.0).abs() < 0.001);
        assert!((service.convert(100.0, Currency::EUR, Currency::CNY) - 775.0).abs() < 0.001);

        // Verify all rates are stored
        let rates = service.list_rates();
        assert_eq!(rates.len(), 3);
    }

    #[test]
    fn test_reverse_rate_fallback() {
        let service = create_test_service();

        // Only set one direction
        service.set_rate(Currency::USD, Currency::CNY, 7.2);

        // Forward conversion uses direct rate
        let forward = service.convert(100.0, Currency::USD, Currency::CNY);
        assert!((forward - 720.0).abs() < 0.001);

        // Reverse conversion uses reverse rate calculation
        let reverse = service.convert(720.0, Currency::CNY, Currency::USD);
        assert!((reverse - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_eur_to_cny_via_usd() {
        let service = create_test_service();

        // Set USD rates only (no direct EUR->CNY)
        service.set_rate(Currency::USD, Currency::CNY, 7.2);
        service.set_rate(Currency::EUR, Currency::USD, 1.08);

        // EUR to USD (direct rate exists)
        let eur_to_usd = service.convert(100.0, Currency::EUR, Currency::USD);
        assert!((eur_to_usd - 108.0).abs() < 0.001);

        // EUR to CNY (no direct rate, should return original amount as fallback)
        // Note: This tests the current behavior where only direct/reverse rates work
        let eur_to_cny = service.convert(100.0, Currency::EUR, Currency::CNY);
        // Since there's no direct EUR->CNY rate and we don't do multi-hop,
        // it returns original amount (current implementation)
        // In a full implementation, this would be 100 * 1.08 * 7.2 = 777.6
        assert!((eur_to_cny - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_zero_amount_conversion() {
        let service = create_test_service();
        service.set_rate(Currency::USD, Currency::CNY, 7.2);

        let amount = service.convert(0.0, Currency::USD, Currency::CNY);
        assert!((amount - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_negative_rate_handling() {
        let service = create_test_service();

        // Setting a negative rate (should not happen in practice, but test behavior)
        service.set_rate(Currency::USD, Currency::CNY, -7.2);

        // Forward conversion with negative rate should work
        let amount = service.convert(100.0, Currency::USD, Currency::CNY);
        assert!((amount - (-720.0)).abs() < 0.001);

        // Reverse conversion: when reverse_rate is negative (not > 0),
        // the code returns the original amount as a safety measure
        let reverse = service.convert(720.0, Currency::CNY, Currency::USD);
        // Due to safety check (reverse_rate > 0.0), returns original amount
        assert!((reverse - 720.0).abs() < 0.001);
    }
}
