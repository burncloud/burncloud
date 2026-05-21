//! HTTP connection pool configuration and management.
//!
//! This module implements HTTP/2 connection pooling and keepalive optimization
//! as described in Issue #251.
//!
//! Features:
//! - HTTP/2 prior knowledge for multiplexing
//! - Connection pool with configurable idle timeout
//! - TCP keepalive for long-lived connections
//! - Per-channel connection pool isolation
//! - Prometheus metrics for pool monitoring

use once_cell::sync::Lazy;
use prometheus::{IntGauge, IntGaugeVec, Registry};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Default pool configuration values.
const DEFAULT_POOL_MAX_IDLE_PER_HOST: usize = 20;
const DEFAULT_POOL_IDLE_TIMEOUT_SECS: u64 = 90;
const DEFAULT_TCP_KEEPALIVE_SECS: u64 = 30;
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 300;

/// Global flag indicating whether HTTP pool metrics are enabled.
static POOL_METRICS_ENABLED: AtomicBool = AtomicBool::new(true);

/// Check if pool metrics collection is enabled.
pub fn is_pool_metrics_enabled() -> bool {
    POOL_METRICS_ENABLED.load(Ordering::Relaxed)
}

// ============================================================================
// Pool Metrics
// ============================================================================

/// Pool metrics registry.
pub static POOL_REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

/// Active connections per channel.
#[allow(clippy::expect_used)]
pub static POOL_CONNECTIONS_ACTIVE: Lazy<IntGaugeVec> = Lazy::new(|| {
    let gauge = IntGaugeVec::new(
        prometheus::Opts::new(
            "burncloud_pool_connections_active",
            "Number of active connections per channel",
        )
        .namespace("burncloud"),
        &["channel_id", "channel_name"],
    )
    .expect("Failed to create POOL_CONNECTIONS_ACTIVE gauge");
    POOL_REGISTRY
        .register(Box::new(gauge.clone()))
        .expect("Failed to register POOL_CONNECTIONS_ACTIVE");
    gauge
});

/// Idle connections per channel.
#[allow(clippy::expect_used)]
pub static POOL_CONNECTIONS_IDLE: Lazy<IntGaugeVec> = Lazy::new(|| {
    let gauge = IntGaugeVec::new(
        prometheus::Opts::new(
            "burncloud_pool_connections_idle",
            "Number of idle connections per channel",
        )
        .namespace("burncloud"),
        &["channel_id", "channel_name"],
    )
    .expect("Failed to create POOL_CONNECTIONS_IDLE gauge");
    POOL_REGISTRY
        .register(Box::new(gauge.clone()))
        .expect("Failed to register POOL_CONNECTIONS_IDLE");
    gauge
});

/// Total pool connections.
#[allow(clippy::expect_used)]
pub static POOL_CONNECTIONS_TOTAL: Lazy<IntGauge> = Lazy::new(|| {
    let gauge = IntGauge::new(
        "burncloud_pool_connections_total",
        "Total number of connections across all pools",
    )
    .expect("Failed to create POOL_CONNECTIONS_TOTAL gauge");
    POOL_REGISTRY
        .register(Box::new(gauge.clone()))
        .expect("Failed to register POOL_CONNECTIONS_TOTAL");
    gauge
});

/// Pool wait time in milliseconds (approximate).
#[allow(clippy::expect_used)]
pub static POOL_WAIT_TIME_MS: Lazy<IntGaugeVec> = Lazy::new(|| {
    let gauge = IntGaugeVec::new(
        prometheus::Opts::new(
            "burncloud_pool_wait_time_ms",
            "Approximate wait time for connection in milliseconds",
        )
        .namespace("burncloud"),
        &["channel_id"],
    )
    .expect("Failed to create POOL_WAIT_TIME_MS gauge");
    POOL_REGISTRY
        .register(Box::new(gauge.clone()))
        .expect("Failed to register POOL_WAIT_TIME_MS");
    gauge
});

// ============================================================================
// Pool Configuration
// ============================================================================

/// Connection pool configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum idle connections per host.
    pub max_idle_per_host: usize,
    /// Idle connection timeout in seconds.
    pub idle_timeout_secs: u64,
    /// TCP keepalive interval in seconds.
    pub keepalive_secs: u64,
    /// Connection timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Request timeout in seconds.
    pub request_timeout_secs: u64,
    /// Enable HTTP/2 prior knowledge.
    pub http2_prior_knowledge: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_idle_per_host: DEFAULT_POOL_MAX_IDLE_PER_HOST,
            idle_timeout_secs: DEFAULT_POOL_IDLE_TIMEOUT_SECS,
            keepalive_secs: DEFAULT_TCP_KEEPALIVE_SECS,
            connect_timeout_secs: DEFAULT_CONNECT_TIMEOUT_SECS,
            request_timeout_secs: DEFAULT_REQUEST_TIMEOUT_SECS,
            http2_prior_knowledge: true,
        }
    }
}

impl PoolConfig {
    /// Load configuration from environment variables.
    ///
    /// Environment variables:
    /// - `HTTP_POOL_MAX_IDLE`: Maximum idle connections per host (default: 20)
    /// - `HTTP_POOL_IDLE_TIMEOUT`: Idle timeout in seconds (default: 90)
    /// - `HTTP_KEEPALIVE_INTERVAL`: TCP keepalive interval in seconds (default: 30)
    /// - `HTTP_CONNECT_TIMEOUT`: Connection timeout in seconds (default: 30)
    /// - `HTTP_REQUEST_TIMEOUT`: Request timeout in seconds (default: 300)
    /// - `HTTP2_PRIOR_KNOWLEDGE`: Enable HTTP/2 prior knowledge (default: true)
    pub fn from_env() -> Self {
        let max_idle = std::env::var("HTTP_POOL_MAX_IDLE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(DEFAULT_POOL_MAX_IDLE_PER_HOST);

        let idle_timeout = std::env::var("HTTP_POOL_IDLE_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_POOL_IDLE_TIMEOUT_SECS);

        let keepalive = std::env::var("HTTP_KEEPALIVE_INTERVAL")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_TCP_KEEPALIVE_SECS);

        let connect_timeout = std::env::var("HTTP_CONNECT_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_CONNECT_TIMEOUT_SECS);

        let request_timeout = std::env::var("HTTP_REQUEST_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_REQUEST_TIMEOUT_SECS);

        let http2_prior_knowledge = std::env::var("HTTP2_PRIOR_KNOWLEDGE")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);

        Self {
            max_idle_per_host: max_idle,
            idle_timeout_secs: idle_timeout,
            keepalive_secs: keepalive,
            connect_timeout_secs: connect_timeout,
            request_timeout_secs: request_timeout,
            http2_prior_knowledge,
        }
    }

    /// Create an HTTP client with this configuration.
    pub fn create_client(&self) -> Result<Client, reqwest::Error> {
        let mut builder = Client::builder()
            .pool_max_idle_per_host(self.max_idle_per_host)
            .pool_idle_timeout(Duration::from_secs(self.idle_timeout_secs))
            .tcp_keepalive(Duration::from_secs(self.keepalive_secs))
            .connect_timeout(Duration::from_secs(self.connect_timeout_secs))
            .timeout(Duration::from_secs(self.request_timeout_secs));

        if self.http2_prior_knowledge {
            builder = builder.http2_prior_knowledge();
        }

        builder.build()
    }
}

// ============================================================================
// Channel Pool Manager
// ============================================================================

/// Manages per-channel isolated connection pools.
///
/// Each channel has its own HTTP client instance to isolate connection pools.
/// This prevents a single channel's connection issues from affecting others.
pub struct ChannelPoolManager {
    /// Configuration for all pools.
    config: PoolConfig,
    /// Per-channel HTTP clients.
    pools: RwLock<HashMap<i32, Arc<Client>>>,
    /// Default client for requests without channel affinity.
    default_client: Arc<Client>,
}

impl ChannelPoolManager {
    /// Create a new channel pool manager with the given configuration.
    pub fn new(config: PoolConfig) -> Result<Self, reqwest::Error> {
        let default_client = Arc::new(config.create_client()?);
        log::info!(
            "HTTP pool initialized: max_idle={}, idle_timeout={}s, keepalive={}s, http2={}",
            config.max_idle_per_host,
            config.idle_timeout_secs,
            config.keepalive_secs,
            config.http2_prior_knowledge
        );

        Ok(Self {
            config,
            pools: RwLock::new(HashMap::new()),
            default_client,
        })
    }

    /// Create a manager with default configuration from environment.
    pub fn from_env() -> Result<Self, reqwest::Error> {
        Self::new(PoolConfig::from_env())
    }

    /// Get or create a client for a specific channel.
    ///
    /// If the channel doesn't have a dedicated client yet, one is created.
    /// This enables per-channel connection pool isolation.
    pub async fn get_client(&self, channel_id: i32) -> Arc<Client> {
        // Fast path: check if client already exists
        {
            let pools = self.pools.read().await;
            if let Some(client) = pools.get(&channel_id) {
                return client.clone();
            }
        }

        // Slow path: create new client for channel
        let mut pools = self.pools.write().await;
        // Double-check after acquiring write lock
        if let Some(client) = pools.get(&channel_id) {
            return client.clone();
        }

        // Create new client for this channel
        let client = match self.config.create_client() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                log::warn!("Failed to create client for channel {}: {}", channel_id, e);
                return self.default_client.clone();
            }
        };

        pools.insert(channel_id, client.clone());
        log::debug!(
            "Created isolated connection pool for channel {}",
            channel_id
        );

        // Update metrics
        if is_pool_metrics_enabled() {
            POOL_CONNECTIONS_TOTAL.inc();
        }

        client
    }

    /// Get the default client (for requests without channel affinity).
    pub fn get_default_client(&self) -> Arc<Client> {
        self.default_client.clone()
    }

    /// Remove a channel's client (e.g., when channel is deleted).
    pub async fn remove_channel(&self, channel_id: i32) {
        let mut pools = self.pools.write().await;
        if pools.remove(&channel_id).is_some() {
            log::debug!("Removed connection pool for channel {}", channel_id);
            if is_pool_metrics_enabled() {
                POOL_CONNECTIONS_TOTAL.dec();
            }
        }
    }

    /// Get the number of active channel pools.
    pub async fn pool_count(&self) -> usize {
        self.pools.read().await.len()
    }

    /// Get the pool configuration.
    pub fn config(&self) -> &PoolConfig {
        &self.config
    }
}

// ============================================================================
// Helper Functions (for future use)
// ============================================================================

/// Record pool connection metrics for a channel.
#[allow(dead_code)]
pub fn record_pool_metrics(channel_id: i32, channel_name: &str, active: i64, idle: i64) {
    if is_pool_metrics_enabled() {
        POOL_CONNECTIONS_ACTIVE
            .with_label_values(&[&channel_id.to_string(), channel_name])
            .set(active);
        POOL_CONNECTIONS_IDLE
            .with_label_values(&[&channel_id.to_string(), channel_name])
            .set(idle);
    }
}

/// Record pool wait time for a channel.
#[allow(dead_code)]
pub fn record_pool_wait_time(channel_id: i32, wait_time_ms: i64) {
    if is_pool_metrics_enabled() {
        POOL_WAIT_TIME_MS
            .with_label_values(&[&channel_id.to_string()])
            .set(wait_time_ms);
    }
}

/// Export pool metrics in Prometheus text format.
#[allow(dead_code)]
pub fn export_pool_metrics() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = POOL_REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .unwrap_or_default();
    String::from_utf8(buffer).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_idle_per_host, 20);
        assert_eq!(config.idle_timeout_secs, 90);
        assert_eq!(config.keepalive_secs, 30);
        assert!(config.http2_prior_knowledge);
    }

    #[test]
    fn test_pool_config_from_env() {
        // Test with no env vars set
        let config = PoolConfig::from_env();
        assert_eq!(config.max_idle_per_host, DEFAULT_POOL_MAX_IDLE_PER_HOST);
    }

    #[test]
    fn test_pool_config_create_client() {
        let config = PoolConfig::default();
        let client = config.create_client();
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_channel_pool_manager() {
        let manager = ChannelPoolManager::new(PoolConfig::default()).unwrap();

        // Get default client
        let client = manager.get_default_client();
        assert!(Arc::strong_count(&client) >= 1);

        // Get client for channel 1
        let client1 = manager.get_client(1).await;
        assert!(Arc::strong_count(&client1) >= 1);

        // Same channel should return same client
        let client1_again = manager.get_client(1).await;
        assert!(Arc::ptr_eq(&client1, &client1_again));

        // Different channel should return different client
        let client2 = manager.get_client(2).await;
        assert!(!Arc::ptr_eq(&client1, &client2));

        // Pool count should be 2
        assert_eq!(manager.pool_count().await, 2);
    }

    #[tokio::test]
    async fn test_remove_channel() {
        let manager = ChannelPoolManager::new(PoolConfig::default()).unwrap();

        // Create client for channel
        manager.get_client(1).await;
        assert_eq!(manager.pool_count().await, 1);

        // Remove channel
        manager.remove_channel(1).await;
        assert_eq!(manager.pool_count().await, 0);
    }

    #[test]
    fn test_pool_metrics_enabled() {
        assert!(is_pool_metrics_enabled());
    }

    #[test]
    fn test_export_pool_metrics() {
        let _ = &*POOL_CONNECTIONS_TOTAL;
        let output = export_pool_metrics();
        assert!(output.contains("burncloud_pool_connections"));
    }
}
