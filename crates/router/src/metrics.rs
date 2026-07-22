//! Prometheus metrics for observability.
//!
//! This module provides Prometheus-compatible metrics for monitoring
//! request rates, latencies, token usage, channel health, and system resources.

use once_cell::sync::Lazy;
use prometheus::{HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Registry};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

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
pub static REGISTRY: Lazy<Registry> = Lazy::new(|| Registry::new());

// ============================================================================
// Request Metrics
// ============================================================================

/// Total number of requests processed.
pub static REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let counter = IntCounterVec::new(
        prometheus::Opts::new(
            "burncloud_requests_total",
            "Total number of requests processed",
        ),
        &["status"],
    )
    .expect("Failed to create REQUESTS_TOTAL counter");
    REGISTRY
        .register(Box::new(counter.clone()))
        .expect("Failed to register REQUESTS_TOTAL");
    counter
});

/// Request latency histogram in seconds.
pub static REQUESTS_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram = HistogramVec::new(
        prometheus::HistogramOpts::new(
            "burncloud_requests_duration_seconds",
            "Request latency in seconds",
        )
        .buckets(vec![
            0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0,
        ]),
        &["endpoint", "model"],
    )
    .expect("Failed to create REQUESTS_DURATION_SECONDS histogram");
    REGISTRY
        .register(Box::new(histogram.clone()))
        .expect("Failed to register REQUESTS_DURATION_SECONDS");
    histogram
});

/// Number of requests currently being processed.
pub static REQUESTS_IN_FLIGHT: Lazy<IntGaugeVec> = Lazy::new(|| {
    let gauge = IntGaugeVec::new(
        prometheus::Opts::new(
            "burncloud_requests_in_flight",
            "Number of requests currently being processed",
        ),
        &["endpoint"],
    )
    .expect("Failed to create REQUESTS_IN_FLIGHT gauge");
    REGISTRY
        .register(Box::new(gauge.clone()))
        .expect("Failed to register REQUESTS_IN_FLIGHT");
    gauge
});

/// Requests by model.
pub static REQUESTS_BY_MODEL: Lazy<IntCounterVec> = Lazy::new(|| {
    let counter = IntCounterVec::new(
        prometheus::Opts::new(
            "burncloud_requests_by_model",
            "Number of requests per model",
        ),
        &["model"],
    )
    .expect("Failed to create REQUESTS_BY_MODEL counter");
    REGISTRY
        .register(Box::new(counter.clone()))
        .expect("Failed to register REQUESTS_BY_MODEL");
    counter
});

/// Requests by channel.
pub static REQUESTS_BY_CHANNEL: Lazy<IntCounterVec> = Lazy::new(|| {
    let counter = IntCounterVec::new(
        prometheus::Opts::new(
            "burncloud_requests_by_channel",
            "Number of requests per channel",
        ),
        &["channel_id", "channel_name"],
    )
    .expect("Failed to create REQUESTS_BY_CHANNEL counter");
    REGISTRY
        .register(Box::new(counter.clone()))
        .expect("Failed to register REQUESTS_BY_CHANNEL");
    counter
});

// ============================================================================
// Token Metrics
// ============================================================================

/// Total prompt tokens processed.
pub static TOKENS_PROMPT_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    let counter = IntCounter::new(
        "burncloud_tokens_prompt_total",
        "Total number of prompt tokens processed",
    )
    .expect("Failed to create TOKENS_PROMPT_TOTAL counter");
    REGISTRY
        .register(Box::new(counter.clone()))
        .expect("Failed to register TOKENS_PROMPT_TOTAL");
    counter
});

/// Total completion tokens generated.
pub static TOKENS_COMPLETION_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    let counter = IntCounter::new(
        "burncloud_tokens_completion_total",
        "Total number of completion tokens generated",
    )
    .expect("Failed to create TOKENS_COMPLETION_TOTAL counter");
    REGISTRY
        .register(Box::new(counter.clone()))
        .expect("Failed to register TOKENS_COMPLETION_TOTAL");
    counter
});

/// Total cost in nanodollars.
pub static COST_TOTAL_NANO: Lazy<IntCounter> = Lazy::new(|| {
    let counter = IntCounter::new("burncloud_cost_total_nano", "Total cost in nanodollars")
        .expect("Failed to create COST_TOTAL_NANO counter");
    REGISTRY
        .register(Box::new(counter.clone()))
        .expect("Failed to register COST_TOTAL_NANO");
    counter
});

// ============================================================================
// Channel Health Metrics
// ============================================================================

/// Channel status (1=healthy, 0=unhealthy).
pub static CHANNEL_STATUS: Lazy<IntGaugeVec> = Lazy::new(|| {
    let gauge = IntGaugeVec::new(
        prometheus::Opts::new(
            "burncloud_channel_status",
            "Channel status (1=healthy, 0=unhealthy)",
        ),
        &["channel_id", "channel_name"],
    )
    .expect("Failed to create CHANNEL_STATUS gauge");
    REGISTRY
        .register(Box::new(gauge.clone()))
        .expect("Failed to register CHANNEL_STATUS");
    gauge
});

/// Channel error count.
pub static CHANNEL_ERRORS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let counter = IntCounterVec::new(
        prometheus::Opts::new(
            "burncloud_channel_errors_total",
            "Total number of channel errors",
        ),
        &["channel_id", "channel_name", "error_type"],
    )
    .expect("Failed to create CHANNEL_ERRORS_TOTAL counter");
    REGISTRY
        .register(Box::new(counter.clone()))
        .expect("Failed to register CHANNEL_ERRORS_TOTAL");
    counter
});

/// Channel latency in seconds.
pub static CHANNEL_LATENCY_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram = HistogramVec::new(
        prometheus::HistogramOpts::new(
            "burncloud_channel_latency_seconds",
            "Channel request latency in seconds",
        )
        .buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0]),
        &["channel_id", "channel_name"],
    )
    .expect("Failed to create CHANNEL_LATENCY_SECONDS histogram");
    REGISTRY
        .register(Box::new(histogram.clone()))
        .expect("Failed to register CHANNEL_LATENCY_SECONDS");
    histogram
});

// ============================================================================
// System Resource Metrics
// ============================================================================

/// Service uptime in seconds.
pub static UPTIME_SECONDS: Lazy<IntGauge> = Lazy::new(|| {
    let gauge = IntGauge::new("burncloud_uptime_seconds", "Service uptime in seconds")
        .expect("Failed to create UPTIME_SECONDS gauge");
    REGISTRY
        .register(Box::new(gauge.clone()))
        .expect("Failed to register UPTIME_SECONDS");
    gauge
});

/// Active connections count.
pub static CONNECTIONS_ACTIVE: Lazy<IntGauge> = Lazy::new(|| {
    let gauge = IntGauge::new(
        "burncloud_connections_active",
        "Number of active connections",
    )
    .expect("Failed to create CONNECTIONS_ACTIVE gauge");
    REGISTRY
        .register(Box::new(gauge.clone()))
        .expect("Failed to register CONNECTIONS_ACTIVE");
    gauge
});

/// Memory usage in bytes.
pub static MEMORY_BYTES: Lazy<IntGauge> = Lazy::new(|| {
    let gauge = IntGauge::new("burncloud_memory_bytes", "Memory usage in bytes")
        .expect("Failed to create MEMORY_BYTES gauge");
    REGISTRY
        .register(Box::new(gauge.clone()))
        .expect("Failed to register MEMORY_BYTES");
    gauge
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
        record_request("success");
        let output = export();
        assert!(output.contains("burncloud_requests_total"));
    }
}
