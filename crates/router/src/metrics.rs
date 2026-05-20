//! Prometheus metrics module for BurnCloud Router
//!
//! This module provides metrics collection and exposition for monitoring
//! the router's performance and health.

use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramVec, Opts, Registry,
};
use std::sync::OnceLock;

/// Global metrics registry
static REGISTRY: OnceLock<Metrics> = OnceLock::new();

/// All metrics for the BurnCloud router
#[derive(Clone)]
pub struct Metrics {
    /// Registry for all metrics
    registry: Registry,

    // Request metrics
    /// Total number of requests processed
    pub requests_total: Counter,
    /// Number of requests currently being processed
    pub requests_in_flight: Gauge,
    /// Request duration histogram
    pub requests_duration_seconds: Histogram,
    /// Requests by model
    pub requests_by_model: CounterVec,
    /// Requests by channel
    pub requests_by_channel: CounterVec,

    // Token metrics
    /// Total prompt tokens processed
    pub tokens_prompt_total: Counter,
    /// Total completion tokens processed
    pub tokens_completion_total: Counter,
    /// Total cost in nanodollars
    pub cost_total_nano: Counter,

    // Channel health metrics
    /// Channel status (1=healthy, 0=unhealthy)
    pub channel_status: GaugeVec,
    /// Channel errors total
    pub channel_errors_total: CounterVec,
    /// Channel latency histogram
    pub channel_latency_seconds: HistogramVec,

    // System metrics
    /// Service uptime in seconds
    pub uptime_seconds: Gauge,
    /// Router errors total
    pub errors_total: CounterVec,
}

impl Metrics {
    /// Create a new metrics instance with all metrics registered
    fn new() -> Self {
        let registry = Registry::new();

        // Request metrics
        let requests_total = Counter::with_opts(Opts::new(
            "burncloud_requests_total",
            "Total number of requests processed",
        ))
        .expect("Failed to create requests_total counter");

        let requests_in_flight = Gauge::with_opts(Opts::new(
            "burncloud_requests_in_flight",
            "Number of requests currently being processed",
        ))
        .expect("Failed to create requests_in_flight gauge");

        let requests_duration_seconds = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "burncloud_requests_duration_seconds",
                "Request duration in seconds",
            )
            .buckets(vec![
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0,
            ]),
        )
        .expect("Failed to create requests_duration_seconds histogram");

        let requests_by_model = CounterVec::new(
            Opts::new(
                "burncloud_requests_by_model",
                "Number of requests by model",
            ),
            &["model"],
        )
        .expect("Failed to create requests_by_model counter");

        let requests_by_channel = CounterVec::new(
            Opts::new(
                "burncloud_requests_by_channel",
                "Number of requests by channel",
            ),
            &["channel_id", "channel_name"],
        )
        .expect("Failed to create requests_by_channel counter");

        // Token metrics
        let tokens_prompt_total = Counter::with_opts(Opts::new(
            "burncloud_tokens_prompt_total",
            "Total prompt tokens processed",
        ))
        .expect("Failed to create tokens_prompt_total counter");

        let tokens_completion_total = Counter::with_opts(Opts::new(
            "burncloud_tokens_completion_total",
            "Total completion tokens processed",
        ))
        .expect("Failed to create tokens_completion_total counter");

        let cost_total_nano = Counter::with_opts(Opts::new(
            "burncloud_cost_total_nano",
            "Total cost in nanodollars",
        ))
        .expect("Failed to create cost_total_nano counter");

        // Channel health metrics
        let channel_status = GaugeVec::new(
            Opts::new(
                "burncloud_channel_status",
                "Channel status (1=healthy, 0=unhealthy)",
            ),
            &["channel_id", "channel_name"],
        )
        .expect("Failed to create channel_status gauge");

        let channel_errors_total = CounterVec::new(
            Opts::new(
                "burncloud_channel_errors_total",
                "Total errors by channel",
            ),
            &["channel_id", "channel_name", "error_type"],
        )
        .expect("Failed to create channel_errors_total counter");

        let channel_latency_seconds = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "burncloud_channel_latency_seconds",
                "Channel request latency in seconds",
            )
            .buckets(vec![
                0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0,
            ]),
            &["channel_id", "channel_name"],
        )
        .expect("Failed to create channel_latency_seconds histogram");

        // System metrics
        let uptime_seconds = Gauge::with_opts(Opts::new(
            "burncloud_uptime_seconds",
            "Service uptime in seconds",
        ))
        .expect("Failed to create uptime_seconds gauge");

        let errors_total = CounterVec::new(
            Opts::new("burncloud_errors_total", "Total errors by type"),
            &["error_type"],
        )
        .expect("Failed to create errors_total counter");

        // Register all metrics
        registry
            .register(Box::new(requests_total.clone()))
            .expect("Failed to register requests_total");
        registry
            .register(Box::new(requests_in_flight.clone()))
            .expect("Failed to register requests_in_flight");
        registry
            .register(Box::new(requests_duration_seconds.clone()))
            .expect("Failed to register requests_duration_seconds");
        registry
            .register(Box::new(requests_by_model.clone()))
            .expect("Failed to register requests_by_model");
        registry
            .register(Box::new(requests_by_channel.clone()))
            .expect("Failed to register requests_by_channel");
        registry
            .register(Box::new(tokens_prompt_total.clone()))
            .expect("Failed to register tokens_prompt_total");
        registry
            .register(Box::new(tokens_completion_total.clone()))
            .expect("Failed to register tokens_completion_total");
        registry
            .register(Box::new(cost_total_nano.clone()))
            .expect("Failed to register cost_total_nano");
        registry
            .register(Box::new(channel_status.clone()))
            .expect("Failed to register channel_status");
        registry
            .register(Box::new(channel_errors_total.clone()))
            .expect("Failed to register channel_errors_total");
        registry
            .register(Box::new(channel_latency_seconds.clone()))
            .expect("Failed to register channel_latency_seconds");
        registry
            .register(Box::new(uptime_seconds.clone()))
            .expect("Failed to register uptime_seconds");
        registry
            .register(Box::new(errors_total.clone()))
            .expect("Failed to register errors_total");

        Self {
            registry,
            requests_total,
            requests_in_flight,
            requests_duration_seconds,
            requests_by_model,
            requests_by_channel,
            tokens_prompt_total,
            tokens_completion_total,
            cost_total_nano,
            channel_status,
            channel_errors_total,
            channel_latency_seconds,
            uptime_seconds,
            errors_total,
        }
    }

    /// Export metrics in Prometheus text format
    pub fn export(&self) -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder
            .encode(&metric_families, &mut buffer)
            .expect("Failed to encode metrics");
        String::from_utf8(buffer).expect("Failed to convert metrics to string")
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Get or initialize the global metrics instance
pub fn metrics() -> &'static Metrics {
    REGISTRY.get_or_init(Metrics::new)
}

/// Export all metrics in Prometheus text format
pub fn export() -> String {
    metrics().export()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let m = Metrics::new();
        // Verify metrics are created and registered
        m.requests_total.inc();
        assert!(m.export().contains("burncloud_requests_total"));
    }

    #[test]
    fn test_requests_by_model() {
        let m = Metrics::new();
        m.requests_by_model.with_label_values(&["gpt-4"]).inc();
        let output = m.export();
        assert!(output.contains("burncloud_requests_by_model"));
        assert!(output.contains("gpt-4"));
    }

    #[test]
    fn test_channel_status() {
        let m = Metrics::new();
        m.channel_status
            .with_label_values(&["1", "openai-main"])
            .set(1.0);
        let output = m.export();
        assert!(output.contains("burncloud_channel_status"));
        assert!(output.contains("openai-main"));
    }

    #[test]
    fn test_histogram() {
        let m = Metrics::new();
        m.requests_duration_seconds.observe(0.5);
        m.requests_duration_seconds.observe(1.0);
        let output = m.export();
        assert!(output.contains("burncloud_requests_duration_seconds"));
    }
}
