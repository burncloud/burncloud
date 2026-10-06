//! Exchange-rate service with an in-memory cache backed by the billing database.

use std::str::FromStr;
use std::sync::Arc;

#[cfg(test)]
use burncloud_commerce_contracts::price_u64::rate_to_scaled;
use burncloud_commerce_contracts::price_u64::scaled_to_rate;
use burncloud_commerce_contracts::pricing::Currency;
use burncloud_database::{sqlx, Database};
use chrono::{DateTime, Utc};
use dashmap::DashMap;

const SYNC_CHECK_INTERVAL_SECS: u64 = 3600;
const STALE_THRESHOLD_HOURS: i64 = 24;
#[cfg(feature = "exchange-api")]
const API_TIMEOUT_SECS: u64 = 5;

#[derive(Debug, Clone)]
pub(crate) struct CachedRate {
    pub rate_nano: i64,
    pub updated_at: DateTime<Utc>,
}

impl CachedRate {
    pub(crate) fn rate(&self) -> f64 {
        scaled_to_rate(self.rate_nano)
    }

    #[cfg(test)]
    pub(crate) fn from_rate(rate: f64) -> Self {
        Self {
            rate_nano: rate_to_scaled(rate),
            updated_at: Utc::now(),
        }
    }
}

pub struct ExchangeRateService {
    db: Arc<Database>,
    rates: DashMap<(Currency, Currency), CachedRate>,
}

impl ExchangeRateService {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            rates: DashMap::new(),
        }
    }

    pub fn convert(&self, amount: f64, from: Currency, to: Currency) -> anyhow::Result<f64> {
        if from == to {
            return Ok(amount);
        }
        if let Some(rate) = self.get_rate(from, to) {
            return Ok(amount * rate);
        }
        if let Some(reverse_rate) = self.get_rate(to, from) {
            if reverse_rate > 0.0 {
                return Ok(amount / reverse_rate);
            }
            return Err(anyhow::anyhow!(
                "Invalid reverse exchange rate for {} -> {}",
                to,
                from
            ));
        }
        Err(anyhow::anyhow!(
            "No exchange rate found for {} -> {}",
            from,
            to
        ))
    }

    pub fn get_rate(&self, from: Currency, to: Currency) -> Option<f64> {
        if from == to {
            return Some(1.0);
        }
        self.rates.get(&(from, to)).map(|rate| rate.rate())
    }

    #[cfg(test)]
    pub(crate) fn set_rate(&self, from: Currency, to: Currency, rate: f64) {
        self.rates.insert((from, to), CachedRate::from_rate(rate));
    }

    pub async fn load_rates_from_db(&self) -> anyhow::Result<usize> {
        let conn = self.db.get_connection()?;
        let rows = sqlx::query_as::<_, (String, String, i64, Option<i64>)>(
            "SELECT from_currency, to_currency, rate, updated_at FROM billing_exchange_rates",
        )
        .fetch_all(conn.pool())
        .await?;

        let mut count = 0;
        for (from, to, rate_nano, updated_at) in rows {
            if let (Ok(from_currency), Ok(to_currency)) =
                (Currency::from_str(&from), Currency::from_str(&to))
            {
                let updated_at = updated_at
                    .map(|timestamp| {
                        DateTime::from_timestamp(timestamp, 0).unwrap_or_else(Utc::now)
                    })
                    .unwrap_or_else(Utc::now);
                self.rates.insert(
                    (from_currency, to_currency),
                    CachedRate {
                        rate_nano,
                        updated_at,
                    },
                );
                count += 1;
            }
        }
        tracing::info!("Loaded {} exchange rates from database", count);
        Ok(count)
    }

    pub fn list_rates(&self) -> Vec<(Currency, Currency, f64, DateTime<Utc>)> {
        self.rates
            .iter()
            .map(|entry| {
                let key = entry.key();
                (key.0, key.1, entry.rate(), entry.updated_at)
            })
            .collect()
    }

    pub fn clear_cache(&self) {
        self.rates.clear();
    }

    #[cfg(test)]
    pub(crate) fn get_last_updated(&self, from: Currency, to: Currency) -> Option<DateTime<Utc>> {
        self.rates.get(&(from, to)).map(|rate| rate.updated_at)
    }

    pub fn start_sync_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(SYNC_CHECK_INTERVAL_SECS));
            loop {
                interval.tick().await;
                match self.load_rates_from_db().await {
                    Ok(count) => tracing::debug!("Loaded {} exchange rates from database", count),
                    Err(error) => {
                        tracing::warn!("Failed to load exchange rates from database: {}", error)
                    }
                }

                let now = Utc::now();
                let needs_refresh = self.rates.iter().any(|entry| {
                    now.signed_duration_since(entry.updated_at).num_hours()
                        >= STALE_THRESHOLD_HOURS
                });
                if needs_refresh {
                    tracing::info!("Exchange rates are stale, attempting refresh");
                    tracing::info!(
                        "Exchange rate auto-refresh not configured. Use 'burncloud currency set-rate' to update manually."
                    );
                }
            }
        });
    }

    #[cfg(feature = "exchange-api")]
    pub async fn fetch_from_api(&self, api_url: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(API_TIMEOUT_SECS))
            .build()?;
        let json: serde_json::Value = client.get(api_url).send().await?.json().await?;

        if let Some(object) = json.as_object() {
            for (key, value) in object {
                let mut parts = key.split('_');
                let Some(from) = parts.next() else {
                    continue;
                };
                let Some(to) = parts.next() else {
                    continue;
                };
                if let (Ok(from_currency), Ok(to_currency), Some(rate)) = (
                    Currency::from_str(from),
                    Currency::from_str(to),
                    value.as_f64(),
                ) {
                    self.set_rate(from_currency, to_currency, rate);
                    tracing::info!(
                        "Updated rate: {} -> {} = {}",
                        from_currency,
                        to_currency,
                        rate
                    );
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::unnecessary_cast,
    clippy::let_and_return,
    clippy::redundant_pattern_matching,
    reason = "tests use direct fixture assertions and isolated process environment setup"
)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    fn test_sqlite_url(path: &str) -> String {
        let normalised = path.replace('\\', "/");
        let absolute = normalised.starts_with('/') || {
            let head: Vec<char> = normalised.chars().take(3).collect();
            head.len() == 3 && head[0].is_ascii_alphabetic() && head[1] == ':' && head[2] == '/'
        };
        if absolute {
            format!("sqlite:///{}?mode=rwc", normalised.trim_start_matches('/'))
        } else {
            format!("sqlite://{}?mode=rwc", normalised)
        }
    }

    fn remove_stale_database(path: &str) {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("failed to remove stale test database {path}: {error}"),
        }
    }

    fn create_test_service() -> ExchangeRateService {
        let _lock = TEST_MUTEX
            .lock()
            .unwrap_or_else(|error| panic!("failed to acquire test mutex: {error}"));
        let runtime = tokio::runtime::Runtime::new()
            .unwrap_or_else(|error| panic!("failed to create tokio runtime: {error}"));
        let db = runtime.block_on(async {
            let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
            let pid = std::process::id();
            let db_path = std::env::temp_dir()
                .join(format!("burncloud_test_exch_{pid}_{test_id}.db"))
                .to_string_lossy()
                .to_string();
            remove_stale_database(&db_path);
            std::env::set_var("BURNCLOUD_DATABASE_URL", test_sqlite_url(&db_path));
            Database::new()
                .await
                .unwrap_or_else(|error| panic!("failed to create test database: {error}"))
        });
        ExchangeRateService::new(Arc::new(db))
    }

    #[test]
    fn test_convert_same_currency() {
        let service = create_test_service();
        let amount = service
            .convert(100.0, Currency::USD, Currency::USD)
            .unwrap();
        assert!((amount - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_set_and_get_rate() {
        let service = create_test_service();
        service.set_rate(Currency::USD, Currency::CNY, 7.2);
        assert_eq!(service.get_rate(Currency::USD, Currency::CNY), Some(7.2));
        assert_eq!(service.get_rate(Currency::CNY, Currency::USD), None);
    }

    #[test]
    fn test_convert_with_rate() {
        let service = create_test_service();
        service.set_rate(Currency::USD, Currency::CNY, 7.2);
        let forward = service
            .convert(100.0, Currency::USD, Currency::CNY)
            .unwrap();
        assert!((forward - 720.0).abs() < 0.001);
        let reverse = service
            .convert(720.0, Currency::CNY, Currency::USD)
            .unwrap();
        assert!((reverse - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_convert_missing_rate() {
        let service = create_test_service();
        assert!(service
            .convert(100.0, Currency::USD, Currency::EUR)
            .is_err());
    }

    #[test]
    fn test_list_rates() {
        let service = create_test_service();
        service.set_rate(Currency::USD, Currency::CNY, 7.2);
        service.set_rate(Currency::EUR, Currency::USD, 1.08);
        assert_eq!(service.list_rates().len(), 2);
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
        assert!(service
            .get_last_updated(Currency::USD, Currency::CNY)
            .is_none());
        service.set_rate(Currency::USD, Currency::CNY, 7.2);
        assert!(service
            .get_last_updated(Currency::USD, Currency::CNY)
            .is_some());
    }

    #[test]
    fn test_multiple_currencies() {
        let service = create_test_service();
        service.set_rate(Currency::USD, Currency::CNY, 7.2);
        service.set_rate(Currency::USD, Currency::EUR, 0.93);
        service.set_rate(Currency::EUR, Currency::CNY, 7.75);
        assert!((service.convert(100.0, Currency::USD, Currency::CNY).unwrap() - 720.0).abs() < 0.001);
        assert!((service.convert(100.0, Currency::USD, Currency::EUR).unwrap() - 93.0).abs() < 0.001);
        assert!((service.convert(100.0, Currency::EUR, Currency::CNY).unwrap() - 775.0).abs() < 0.001);
        assert_eq!(service.list_rates().len(), 3);
    }

    #[test]
    fn test_reverse_rate_fallback() {
        let service = create_test_service();
        service.set_rate(Currency::USD, Currency::CNY, 7.2);
        let forward = service
            .convert(100.0, Currency::USD, Currency::CNY)
            .unwrap();
        assert!((forward - 720.0).abs() < 0.001);
        let reverse = service
            .convert(720.0, Currency::CNY, Currency::USD)
            .unwrap();
        assert!((reverse - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_eur_to_cny_via_usd() {
        let service = create_test_service();
        service.set_rate(Currency::USD, Currency::CNY, 7.2);
        service.set_rate(Currency::EUR, Currency::USD, 1.08);
        let eur_to_usd = service
            .convert(100.0, Currency::EUR, Currency::USD)
            .unwrap();
        assert!((eur_to_usd - 108.0).abs() < 0.001);
        assert!(service
            .convert(100.0, Currency::EUR, Currency::CNY)
            .is_err());
    }

    #[test]
    fn test_zero_amount_conversion() {
        let service = create_test_service();
        service.set_rate(Currency::USD, Currency::CNY, 7.2);
        let amount = service.convert(0.0, Currency::USD, Currency::CNY).unwrap();
        assert!(amount.abs() < 0.001);
    }

    #[test]
    fn test_negative_rate_handling() {
        let service = create_test_service();
        service.set_rate(Currency::USD, Currency::CNY, -7.2);
        let amount = service
            .convert(100.0, Currency::USD, Currency::CNY)
            .unwrap();
        assert!((amount + 720.0).abs() < 0.001);
        assert!(service
            .convert(720.0, Currency::CNY, Currency::USD)
            .is_err());
    }
}
