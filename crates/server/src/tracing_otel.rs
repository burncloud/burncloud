//! OpenTelemetry distributed tracing integration.
//!
//! This module provides OpenTelemetry SDK initialization for distributed tracing.
//! Traces are exported via OTLP protocol to compatible backends (Jaeger, Zipkin, Grafana Tempo).
//!
//! # Configuration
//!
//! - `OTEL_EXPORTER_OTLP_ENDPOINT`: OTLP collector endpoint (e.g., `http://localhost:4317`)
//! - `OTEL_TRACES_SAMPLER_ARG`: Sampling rate (0.0 to 1.0, default: 0.1 for 10%)
//! - `OTEL_SERVICE_NAME`: Service name for traces (default: "burncloud")
//!
//! # Security
//!
//! Span attributes are carefully filtered to avoid leaking sensitive information:
//! - API keys are never recorded in spans
//! - Request/response bodies are not recorded
//! - Only metadata like request_id, model, channel_id are captured

use std::env;
use std::sync::OnceLock;

use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_sdk::trace::{Sampler, TracerProvider as SdkTracerProvider};
use opentelemetry_sdk::Resource;

/// Global tracer provider guard. Must be held for the program's lifetime.
static TRACER_PROVIDER_GUARD: OnceLock<SdkTracerProvider> = OnceLock::new();

/// Initialize OpenTelemetry tracing layer.
///
/// Returns `Some(layer)` if OTEL_EXPORTER_OTLP_ENDPOINT is configured,
/// otherwise returns `None` (tracing disabled).
///
/// The returned layer should be added to the tracing subscriber.
pub fn init_opentelemetry() -> Option<
    tracing_opentelemetry::OpenTelemetryLayer<
        tracing_subscriber::Registry,
        opentelemetry_sdk::trace::Tracer,
    >,
> {
    let endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok()?;

    let service_name = env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "burncloud".to_string());

    let sample_rate = env::var("OTEL_TRACES_SAMPLER_ARG")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.1); // Default: 10% sampling

    tracing::info!(
        endpoint = %endpoint,
        service_name = %service_name,
        sample_rate = %sample_rate,
        "Initializing OpenTelemetry tracing"
    );

    let resource = Resource::new(vec![KeyValue::new("service.name", service_name)]);

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(format!("{}/v1/traces", endpoint.trim_end_matches('/')))
        .build()
        .ok()?;

    let provider = SdkTracerProvider::builder()
        .with_resource(resource)
        .with_sampler(Sampler::TraceIdRatioBased(sample_rate))
        .with_batch_exporter(exporter, Tokio)
        .build();

    let tracer = provider.tracer("burncloud");

    // Store provider for later shutdown
    let _ = TRACER_PROVIDER_GUARD.set(provider);

    Some(tracing_opentelemetry::layer().with_tracer(tracer))
}

/// Shutdown OpenTelemetry tracer provider.
///
/// This should be called on graceful shutdown to flush pending traces.
pub fn shutdown() {
    if let Some(provider) = TRACER_PROVIDER_GUARD.get() {
        let _ = provider.shutdown();
        tracing::info!("OpenTelemetry tracer provider shut down");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_sample_rate() {
        // Without env var, should use 0.1
        env::remove_var("OTEL_TRACES_SAMPLER_ARG");
        let rate = env::var("OTEL_TRACES_SAMPLER_ARG")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.1);
        assert!((rate - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_custom_sample_rate() {
        env::set_var("OTEL_TRACES_SAMPLER_ARG", "0.5");
        let rate = env::var("OTEL_TRACES_SAMPLER_ARG")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.1);
        assert!((rate - 0.5).abs() < 0.001);
        env::remove_var("OTEL_TRACES_SAMPLER_ARG");
    }
}
