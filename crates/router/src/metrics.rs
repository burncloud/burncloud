//! Prometheus metrics for observability.
//!
//! This module provides Prometheus-compatible metrics for monitoring
//! request rates, latencies, token usage, channel health, and system resources.

use once_cell::sync::Lazy;
use prometheus::{HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Registry};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// Helper macro: safely create and register a CounterVec metric.
macro_rules! create_register_counter_vec {
    ($name:expr, $help:expr, $labels:expr) => {{
        let counter = IntCounterVec::new(
            prometheus::Opts::new($name, $help).namespace("burncloud"),
            $labels,
        )
        .unwrap_or_else(|e| panic!(
            "Failed to create Prometheus IntCounterVec '{}': {}. \
             This indicates a metric name conflict. \
             Please ensure all metric names are unique across the entire application.",
            $name, e
        ));
        REGISTRY
            .register(Box::new(counter.clone()))
            .unwrap_or_else(|e| panic!(
                "Failed to register Prometheus IntCounterVec '{}': {}. \
                 This indicates a metric name conflict.",
                $name, e
            ));
        counter
    }};
}

/// Helper macro: safely create and register a HistogramVec metric.
macro_rules! create_register_histogram_vec {
    ($name:expr, $help:expr, $buckets:expr, $labels:expr) => {{
        let histogram = HistogramVec::new(
            prometheus::HistogramOpts::new($name, $help)
                .namespace("burncloud")
                .buckets($buckets),
            $labels,
        )
        .unwrap_or_else(|e| panic!(
            "Failed to create Prometheus HistogramVec '{}': {}. \
             This indicates a metric name conflict. \
             Please ensure all metric names are unique across the entire application.",
            $name, e
        ));
        REGISTRY
            .register(Box::new(histogram.clone()))
            .unwrap_or_else(|e| panic!(
                "Failed to register Prometheus HistogramVec '{}': {}. \
                 This indicates a metric name conflict.",
                $name, e
            ));
        histogram
    }};
}

/// Helper macro: safely create and register a GaugeVec metric.
macro_rules! create_register_gauge_vec {
    ($name:expr, $help:expr, $labels:expr) => {{
        let gauge = IntGaugeVec::new(
            prometheus::Opts::new($name, $help).namespace("burncloud"),
            $labels,
        )
        .unwrap_or_else(|e| panic!(
            "Failed to create Prometheus IntGaugeVec '{}': {}. \
             This indicates a metric name conflict. \
             Please ensure all metric names are unique across the entire application.",
            $name, e
        ));
        REGISTRY
            .register(Box::new(gauge.clone()))
            .unwrap_or_else(|e| panic!(
                "Failed to register Prometheus IntGaugeVec '{}': {}. \
                 This indicates a metric name conflict.",
                $name, e
            ));
        gauge
    }};
}

/// Helper macro: safely create and register a Counter metric.
macro_rules! create_register_counter {
    ($name:expr, $help:expr) => {{
        let counter = IntCounter::new($name, $help)
            .unwrap_or_else(|e| panic!(
                "Failed to create Prometheus IntCounter '{}': {}. \
                 This indicates a metric name conflict. \
                 Please ensure all metric names are unique across the entire application.",
                $name, e
            ));
        REGISTRY
            .register(Box::new(counter.clone()))
            .unwrap_or_else(|e| panic!(
                "Failed to register Prometheus IntCounter '{}': {}. \
                 This indicates a metric name conflict.",
                $name, e
            ));
        counter
    }};
}

/// Helper macro: safely create and register a Gauge metric.
macro_rules! create_register_gauge {
    ($name:expr, $help:expr) => {{
        let gauge = IntGauge::new($name, $help)
            .unwrap_or_else(|e| panic!(
                "Failed to create Prometheus IntGauge '{}': {}. \
                 This indicates a metric name conflict. \
                 Please ensure all metric names are unique across the entire application.",
                $name, e
            ));
        REGISTRY
            .register(Box::new(gauge.clone()))
            .unwrap_or_else(|e| panic!(
                "Failed to register Prometheus IntGauge '{}': {}. \
                 This indicates a metric name conflict.",
                $name, e
            ));
        gauge
    }};
}

/// Global flag indicating whether metrics collection is enabled.
static METRICS_ENABLED: AtomicBool = AtomicBool::new(true);

/// Check if metrics collection is enabled.
pub fn is_enabled() -> bool {
    METRICS_ENABLED.load(Ordering::Relaxed)
}

/// Enable or disable metrics collection.
pub fn set_enabled(enabled: bool) {
    METRICS_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Initialize metrics from environment variable.
pub fn init_from_env() {
    let enabled = std::env::var("METRICS_ENABLED")
        .map(|v| v != "false" && v != "0")
        .unwrap_or(true);
    set_enabled(enabled);
    if enabled {
        log::info!("Prometheus metrics enabled");
    } else {
        log::info!("Prometheus metrics disabled via METRICS_ENABLED=false");
    }
}

/// Custom Prometheus registry for burncloud metrics.
pub static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

// ============================================================================
// Request Metrics
// ============================================================================

/// Total number of requests processed.
pub static REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    create_register_counter_vec!("burncloud_requests_total", "Total number of requests processed", &["status"])
});

/// Request latency histogram in seconds.
pub static REQUESTS_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    create_register_histogram_vec!(
        "burncloud_requests_duration_seconds",
        "Request latency in seconds",
        vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0],
        &["endpoint", "model"]
    )
});

/// Number of requests currently being processed.
pub static REQUESTS_IN_FLIGHT: Lazy<IntGaugeVec> = Lazy::new(|| {
    create_register_gauge_vec!("burncloud_requests_in_flight", "Number of requests currently being processed", &["endpoint"])
});

/// Requests by model.
pub static REQUESTS_BY_MODEL: Lazy<IntCounterVec> = Lazy::new(|| {
    create_register_counter_vec!("burncloud_requests_by_model", "Number of requests per model", &["model"])
});

/// Requests by channel.
pub static REQUESTS_BY_CHANNEL: Lazy<IntCounterVec> = Lazy::new(|| {
    create_register_counter_vec!("burncloud_requests_by_channel", "Number of requests per channel", &["channel_id", "channel_name"])
});

// ============================================================================
// Token Metrics
// ============================================================================

/// Total prompt tokens processed.
pub static TOKENS_PROMPT_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    create_register_counter!("burncloud_tokens_prompt_total", "Total number of prompt tokens processed")
});

/// Total completion tokens generated.
pub static TOKENS_COMPLETION_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    create_register_counter!("burncloud_tokens_completion_total", "Total number of completion tokens generated")
});

/// Total cost in nanodollars.
pub static COST_TOTAL_NANO: Lazy<IntCounter> = Lazy::new(|| {
    create_register_counter!("burncloud_cost_total_nano", "Total cost in nanodollars")
});

// ============================================================================
// Channel Health Metrics
// ============================================================================

/// Channel status (1=healthy, 0=unhealthy).
pub static CHANNEL_STATUS: Lazy<IntGaugeVec> = Lazy::new(|| {
    create_register_gauge_vec!("burncloud_channel_status", "Channel status (1=healthy, 0=unhealthy)", &["channel_id", "channel_name"])
});

/// Channel error count.
pub static CHANNEL_ERRORS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    create_register_counter_vec!("burncloud_channel_errors_total", "Total number of channel errors", &["channel_id", "channel_name", "error_type"])
});

/// Channel latency in seconds.
pub static CHANNEL_LATENCY_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    create_register_histogram_vec!(
        "burncloud_channel_latency_seconds",
        "Channel request latency in seconds",
        vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0],
        &["channel_id", "channel_name"]
    )
});

// ============================================================================
// System Resource Metrics
// ============================================================================

/// Service uptime in seconds.
pub static UPTIME_SECONDS: Lazy<IntGauge> = Lazy::new(|| {
    create_register_gauge!("burncloud_uptime_seconds", "Service uptime in seconds")
});

/// Active connections count.
pub static CONNECTIONS_ACTIVE: Lazy<IntGauge> = Lazy::new(|| {
    create_register_gauge!("burncloud_connections_active", "Number of active connections")
});

/// Memory usage in bytes.
pub static MEMORY_BYTES: Lazy<IntGauge> = Lazy::new(|| {
    create_register_gauge!("burncloud_memory_bytes", "Memory usage in bytes")
});

/// Service start time for uptime calculation.
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

// ============================================================================
// Helper Functions
// ============================================================================

/// Record a request with status.
pub fn record_request(status: &str) {
    if is_enabled() {
        REQUESTS_TOTAL.with_label_values(&[status]).inc();
    }
}

/// Record request duration.
pub fn record_request_duration(endpoint: &str, model: &str, duration_secs: f64) {
    if is_enabled() {
        REQUESTS_DURATION_SECONDS
            .with_label_values(&[endpoint, model])
            .observe(duration_secs);
    }
}

/// Increment in-flight requests.
pub fn inc_in_flight(endpoint: &str) {
    if is_enabled() {
        REQUESTS_IN_FLIGHT.with_label_values(&[endpoint]).inc();
    }
}

/// Decrement in-flight requests.
pub fn dec_in_flight(endpoint: &str) {
    if is_enabled() {
        REQUESTS_IN_FLIGHT.with_label_values(&[endpoint]).dec();
    }
}

/// Record a request by model.
pub fn record_request_by_model(model: &str) {
    if is_enabled() {
        REQUESTS_BY_MODEL.with_label_values(&[model]).inc();
    }
}

/// Record a request by channel.
pub fn record_request_by_channel(channel_id: i32, channel_name: &str) {
    if is_enabled() {
        REQUESTS_BY_CHANNEL
            .with_label_values(&[&channel_id.to_string(), channel_name])
            .inc();
    }
}

/// Record prompt tokens.
pub fn record_prompt_tokens(count: u64) {
    if is_enabled() {
        TOKENS_PROMPT_TOTAL.inc_by(count);
    }
}

/// Record completion tokens.
pub fn record_completion_tokens(count: u64) {
    if is_enabled() {
        TOKENS_COMPLETION_TOTAL.inc_by(count);
    }
}

/// Record cost in nanodollars.
pub fn record_cost_nano(cost_nano: u64) {
    if is_enabled() {
        COST_TOTAL_NANO.inc_by(cost_nano);
    }
}

/// Set channel status.
pub fn set_channel_status(channel_id: i32, channel_name: &str, healthy: bool) {
    if is_enabled() {
        CHANNEL_STATUS
            .with_label_values(&[&channel_id.to_string(), channel_name])
            .set(if healthy { 1 } else { 0 });
    }
}

/// Record a channel error.
pub fn record_channel_error(channel_id: i32, channel_name: &str, error_type: &str) {
    if is_enabled() {
        CHANNEL_ERRORS_TOTAL
            .with_label_values(&[&channel_id.to_string(), channel_name, error_type])
            .inc();
    }
}

/// Record channel latency.
pub fn record_channel_latency(channel_id: i32, channel_name: &str, latency_secs: f64) {
    if is_enabled() {
        CHANNEL_LATENCY_SECONDS
            .with_label_values(&[&channel_id.to_string(), channel_name])
            .observe(latency_secs);
    }
}

/// Update system metrics (uptime, memory).
pub fn update_system_metrics() {
    if is_enabled() {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        UPTIME_SECONDS.set(uptime);

        // Try to get memory usage (best effort)
        #[cfg(target_os = "linux")]
        {
            if let Ok(usage) = get_memory_usage_linux() {
                MEMORY_BYTES.set(usage as i64);
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Ok(usage) = get_memory_usage_macos() {
                MEMORY_BYTES.set(usage as i64);
            }
        }
    }
}

/// Get memory usage on Linux.
#[cfg(target_os = "linux")]
fn get_memory_usage_linux() -> Result<u64, std::io::Error> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open("/proc/self/status")?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                // VmRSS is in kB, convert to bytes
                if let Ok(kb) = parts[1].parse::<u64>() {
                    return Ok(kb * 1024);
                }
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "VmRSS not found in /proc/self/status",
    ))
}

/// Get memory usage on macOS.
#[cfg(target_os = "macos")]
fn get_memory_usage_macos() -> Result<u64, std::io::Error> {
    // On macOS, use task_info to get resident size
    // For simplicity, return 0 if we can't get it
    Ok(0)
}

/// Export metrics in Prometheus text format.
pub fn export() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = REGISTRY.gather();
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
    fn test_metrics_enabled_by_default() {
        assert!(is_enabled());
    }

    #[test]
    fn test_metrics_can_be_disabled() {
        set_enabled(true);
        assert!(is_enabled());

        set_enabled(false);
        assert!(!is_enabled());

        // Reset for other tests
        set_enabled(true);
    }

    #[test]
    fn test_record_request() {
        set_enabled(true);
        record_request("success");
        // Counter should have been incremented
    }

    #[test]
    fn test_record_request_disabled() {
        set_enabled(false);
        record_request("success");
        // Should not panic
        set_enabled(true);
    }

    #[test]
    fn test_export() {
        // Initialize the metrics by accessing them
        let _ = &*REQUESTS_TOTAL;
        set_enabled(true);
        let output = export();
        assert!(output.contains("burncloud_requests_total"));
    }
}
