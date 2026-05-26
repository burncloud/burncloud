//! Dashboard metrics for real-time WebSocket updates.
//!
//! This module provides aggregated metrics for the real-time dashboard,
//! combining system metrics, request metrics, and channel health.

use serde::{Deserialize, Serialize};

/// Dashboard metrics for WebSocket broadcast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    /// Message type identifier
    #[serde(rename = "type")]
    pub msg_type: String,
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Traffic metrics
    pub data: DashboardData,
}

impl DashboardMetrics {
    /// Create new dashboard metrics from components.
    pub fn new(
        traffic: TrafficMetrics,
        tokens: TokenMetrics,
        channels: Vec<ChannelHealth>,
        costs: CostMetrics,
    ) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339();
        Self {
            msg_type: "metrics".to_string(),
            timestamp,
            data: DashboardData {
                traffic,
                tokens,
                channels,
                costs,
            },
        }
    }
}

/// Aggregated dashboard data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    /// Traffic metrics (QPS, connections, latency)
    pub traffic: TrafficMetrics,
    /// Token usage metrics
    pub tokens: TokenMetrics,
    /// Channel health status
    pub channels: Vec<ChannelHealth>,
    /// Cost metrics
    pub costs: CostMetrics,
}

/// Traffic-related metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficMetrics {
    /// Current queries per second
    pub qps: f64,
    /// Requests per minute
    pub rpm: u64,
    /// Active connections count
    pub active_connections: u64,
    /// Latency percentiles in milliseconds
    pub latency: LatencyPercentiles,
}

/// Latency percentile distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyPercentiles {
    /// 50th percentile latency (ms)
    pub p50: f64,
    /// 90th percentile latency (ms)
    pub p90: f64,
    /// 99th percentile latency (ms)
    pub p99: f64,
}

/// Token usage metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetrics {
    /// Prompt tokens per second
    pub prompt_rate: f64,
    /// Completion tokens per second
    pub completion_rate: f64,
    /// Total prompt tokens (cumulative)
    pub prompt_total: u64,
    /// Total completion tokens (cumulative)
    pub completion_total: u64,
    /// Token usage by model
    pub by_model: Vec<ModelTokenUsage>,
}

/// Token usage per model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTokenUsage {
    /// Model name
    pub model: String,
    /// Total tokens for this model
    pub tokens: u64,
    /// Percentage of total
    pub percentage: f64,
}

/// Channel health status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelHealth {
    /// Channel ID
    pub id: i32,
    /// Channel name
    pub name: String,
    /// Health status: "healthy", "degraded", "unhealthy"
    pub status: String,
    /// Average latency in milliseconds
    pub latency_ms: f64,
    /// Request count in current window
    pub request_count: u64,
    /// Error count in current window
    pub error_count: u64,
}

/// Cost tracking metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostMetrics {
    /// Total cost in USD
    pub total_usd: f64,
    /// Cost rate (USD per hour)
    pub rate_per_hour: f64,
    /// Cost by channel
    pub by_channel: Vec<ChannelCost>,
}

/// Cost per channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelCost {
    /// Channel name
    pub channel: String,
    /// Cost in USD
    pub cost_usd: f64,
    /// Percentage of total
    pub percentage: f64,
}

/// Dashboard service for collecting and aggregating metrics.
pub struct DashboardService;

impl DashboardService {
    /// Create a sample dashboard metrics (for testing/development).
    /// In production, this would pull from Prometheus metrics and system monitor.
    pub fn sample_metrics() -> DashboardMetrics {
        let traffic = TrafficMetrics {
            qps: 42.5,
            rpm: 2550,
            active_connections: 15,
            latency: LatencyPercentiles {
                p50: 120.0,
                p90: 450.0,
                p99: 850.0,
            },
        };

        let tokens = TokenMetrics {
            prompt_rate: 1500.0,
            completion_rate: 500.0,
            prompt_total: 1_000_000,
            completion_total: 350_000,
            by_model: vec![
                ModelTokenUsage {
                    model: "gpt-4".to_string(),
                    tokens: 500_000,
                    percentage: 37.0,
                },
                ModelTokenUsage {
                    model: "claude-3-opus".to_string(),
                    tokens: 400_000,
                    percentage: 29.6,
                },
                ModelTokenUsage {
                    model: "gpt-3.5-turbo".to_string(),
                    tokens: 450_000,
                    percentage: 33.3,
                },
            ],
        };

        let channels = vec![
            ChannelHealth {
                id: 1,
                name: "OpenAI Official".to_string(),
                status: "healthy".to_string(),
                latency_ms: 150.0,
                request_count: 1000,
                error_count: 5,
            },
            ChannelHealth {
                id: 2,
                name: "Anthropic Direct".to_string(),
                status: "healthy".to_string(),
                latency_ms: 200.0,
                request_count: 500,
                error_count: 2,
            },
            ChannelHealth {
                id: 3,
                name: "Azure OpenAI".to_string(),
                status: "degraded".to_string(),
                latency_ms: 500.0,
                request_count: 200,
                error_count: 15,
            },
        ];

        let costs = CostMetrics {
            total_usd: 125.50,
            rate_per_hour: 15.2,
            by_channel: vec![
                ChannelCost {
                    channel: "OpenAI Official".to_string(),
                    cost_usd: 75.30,
                    percentage: 60.0,
                },
                ChannelCost {
                    channel: "Anthropic Direct".to_string(),
                    cost_usd: 40.15,
                    percentage: 32.0,
                },
                ChannelCost {
                    channel: "Azure OpenAI".to_string(),
                    cost_usd: 10.05,
                    percentage: 8.0,
                },
            ],
        };

        DashboardMetrics::new(traffic, tokens, channels, costs)
    }
}
