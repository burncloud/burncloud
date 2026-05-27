//! OpenTelemetry distributed tracing initialization.
//!
//! This module provides OpenTelemetry integration for distributed tracing,
//! supporting OTLP protocol export to backends like Jaeger, Zipkin, and Grafana Tempo.
//!
//! # Configuration
//!
//! Tracing is configured via environment variables:
//!
//! - `OTEL_EXPORTER_OTLP_ENDPOINT`: OTLP collector endpoint (e.g., `http://localhost:4317`)
//! - `OTEL_TRACES_SAMPLER_ARG`: Sampling rate (0.0 to 1.0, default: 0.1 for 10%)
//! - `OTEL_SERVICE_NAME`: Service name (default: `burncloud`)
//!
//! # Example
//!
//! ```bash
//! # Enable tracing with 10% sampling
//! export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
//! export OTEL_TRACES_SAMPLER_ARG=0.1
//! ```

use opentelemetry::{trace::TracerProvider, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::Sampler;
use std::env;

/// Initialize OpenTelemetry tracing layer.
///
/// Returns `Some(layer)` if OTLP endpoint is configured, `None` otherwise.
/// The returned layer should be added to the tracing subscriber.
pub fn init_telemetry() -> Option<
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
        .unwrap_or(0.1)
        .clamp(0.0, 1.0);

    tracing::info!(
        endpoint = %endpoint,
        service_name = %service_name,
        sample_rate = sample_rate,
        "Initializing OpenTelemetry tracing"
    );

    let resource = opentelemetry_sdk::Resource::new([KeyValue::new(
        opentelemetry_semantic_conventions::resource::SERVICE_NAME,
        service_name,
    )]);

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&endpoint)
        .build()
        .ok()?;

    let tracer_provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_resource(resource)
        .with_sampler(Sampler::TraceIdRatioBased(sample_rate))
        .build();

    let tracer = tracer_provider.tracer("burncloud");

    Some(tracing_opentelemetry::layer().with_tracer(tracer))
}

/// Shutdown OpenTelemetry tracer provider.
///
/// This should be called when the application shuts down to flush
/// any remaining spans to the collector.
pub fn shutdown_telemetry() {
    opentelemetry::global::shutdown_tracer_provider();
}
