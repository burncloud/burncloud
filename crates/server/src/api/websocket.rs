//! WebSocket real-time dashboard.
//!
//! Provides a WebSocket endpoint for pushing real-time metrics to connected clients.
//! Metrics include QPS, active connections, latency, token usage, channel status, and costs.

use crate::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use burncloud_router::metrics;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

/// Global counter for active WebSocket connections.
static ACTIVE_WS_CONNECTIONS: AtomicU64 = AtomicU64::new(0);

/// Channel for broadcasting metrics updates to all WebSocket clients.
static METRICS_CHANNEL: std::sync::OnceLock<broadcast::Sender<DashboardMetrics>> =
    std::sync::OnceLock::new();

/// Get or initialize the metrics broadcast channel.
fn get_metrics_channel() -> &'static broadcast::Sender<DashboardMetrics> {
    METRICS_CHANNEL.get_or_init(|| {
        let (tx, _rx) = broadcast::channel::<DashboardMetrics>(16);
        tx
    })
}

/// Dashboard metrics message sent to WebSocket clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    /// Message type identifier.
    #[serde(rename = "type")]
    pub msg_type: String,
    /// Timestamp of the metrics collection.
    pub timestamp: String,
    /// Traffic metrics (QPS, RPM, connections, latency).
    pub traffic: TrafficMetrics,
    /// Token consumption metrics.
    pub tokens: TokenMetrics,
    /// Channel status metrics.
    pub channels: ChannelMetrics,
    /// System resource metrics.
    pub system: SystemResourceMetrics,
}

/// Traffic-related metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficMetrics {
    /// Current queries per second.
    pub qps: u64,
    /// Requests per minute.
    pub rpm: u64,
    /// Active WebSocket connections.
    pub active_ws_connections: u64,
    /// Requests currently being processed (in-flight).
    pub in_flight: u64,
    /// Latency P50 in milliseconds.
    pub latency_p50: u64,
    /// Latency P99 in milliseconds.
    pub latency_p99: u64,
}

/// Token consumption metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetrics {
    /// Total prompt tokens processed.
    pub prompt_total: u64,
    /// Total completion tokens generated.
    pub completion_total: u64,
    /// Token consumption rate (tokens/sec).
    pub rate_per_sec: u64,
    /// Total cost in microdollars (for display).
    pub cost_total_micro: u64,
}

/// Channel status metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMetrics {
    /// Number of healthy channels.
    pub healthy: u64,
    /// Number of unhealthy channels.
    pub unhealthy: u64,
    /// Total channels configured.
    pub total: u64,
}

/// System resource metrics (from SystemMonitorService).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResourceMetrics {
    /// CPU usage percentage.
    pub cpu_usage_percent: f32,
    /// Memory usage percentage.
    pub memory_usage_percent: f32,
    /// Memory used in formatted string (e.g., "1.2 GB").
    pub memory_used_formatted: String,
    /// Disk usage percentage (average across all disks).
    pub disk_usage_percent: f32,
}

/// WebSocket message types from client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    /// Client requests immediate metrics refresh.
    #[serde(rename = "refresh")]
    Refresh,
    /// Client subscribes to specific metric types.
    #[serde(rename = "subscribe")]
    Subscribe { metrics: Vec<String> },
}

/// Create WebSocket routes.
pub fn routes() -> Router<AppState> {
    Router::new().route("/ws/dashboard", get(handle_dashboard_ws_upgrade))
}

/// Handle WebSocket upgrade request.
#[tracing::instrument(skip(state))]
async fn handle_dashboard_ws_upgrade(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_dashboard_ws(socket, state))
}

/// Handle WebSocket connection for dashboard metrics.
#[tracing::instrument(skip(socket, state))]
async fn handle_dashboard_ws(socket: WebSocket, state: AppState) {
    // Increment connection counter
    ACTIVE_WS_CONNECTIONS.fetch_add(1, Ordering::Relaxed);

    // Split socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to metrics broadcast channel
    let mut metrics_rx = get_metrics_channel().subscribe();

    // Track last metrics update time for periodic pushing
    let mut last_update = Instant::now();

    tracing::info!(
        active_connections = ACTIVE_WS_CONNECTIONS.load(Ordering::Relaxed),
        "WebSocket dashboard client connected"
    );

    // Send initial metrics immediately
    let initial_metrics = collect_dashboard_metrics(&state).await;
    if let Ok(metrics_json) = serde_json::to_string(&initial_metrics) {
        if sender.send(Message::Text(metrics_json.into())).await.is_err() {
            tracing::warn!("Failed to send initial metrics to WebSocket client");
            ACTIVE_WS_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
            return;
        }
    }

    // Main WebSocket loop
    loop {
        tokio::select! {
            // Handle messages from client
            client_msg = receiver.next() => {
                match client_msg {
                    Some(Ok(Message::Text(text))) => {
                        // Parse client message
                        if let Ok(msg) = serde_json::from_str::<ClientMessage>(&text) {
                            match msg {
                                ClientMessage::Refresh => {
                                    // Client requested immediate refresh
                                    let metrics = collect_dashboard_metrics(&state).await;
                                    if let Ok(metrics_json) = serde_json::to_string(&metrics) {
                                        if sender.send(Message::Text(metrics_json.into())).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                                ClientMessage::Subscribe { metrics: _ } => {
                                    // Subscription handling - currently all metrics are pushed
                                    tracing::debug!("Client subscription request received");
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        // Respond to ping with pong
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!("WebSocket client closed connection");
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // Handle broadcast messages from metrics channel
            broadcast_msg = metrics_rx.recv() => {
                match broadcast_msg {
                    Ok(metrics) => {
                        if let Ok(metrics_json) = serde_json::to_string(&metrics) {
                            if sender.send(Message::Text(metrics_json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::warn!("Metrics broadcast channel closed");
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        tracing::debug!("WebSocket client lagged behind metrics broadcast");
                    }
                }
            }

            // Periodic metrics update (every 1 second for high-frequency metrics)
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                if last_update.elapsed() >= Duration::from_secs(1) {
                    let metrics = collect_dashboard_metrics(&state).await;
                    
                    // Broadcast to all connected clients
                    let _ = get_metrics_channel().send(metrics.clone());
                    
                    // Also send directly to this client
                    if let Ok(metrics_json) = serde_json::to_string(&metrics) {
                        if sender.send(Message::Text(metrics_json.into())).await.is_err() {
                            break;
                        }
                    }
                    
                    last_update = Instant::now();
                }
            }
        }
    }

    // Decrement connection counter on exit
    ACTIVE_WS_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
    tracing::info!(
        active_connections = ACTIVE_WS_CONNECTIONS.load(Ordering::Relaxed),
        "WebSocket dashboard client disconnected"
    );
}

/// Collect all dashboard metrics from various sources.
#[tracing::instrument(skip(state))]
async fn collect_dashboard_metrics(state: &AppState) -> DashboardMetrics {
    // Collect system metrics from monitor service
    let system_metrics = state.monitor.get_metrics().await.ok();

    // Get Prometheus metrics (approximate values from counters/gauges)
    let (prompt_total, completion_total, cost_total_nano) = get_token_metrics();
    let (requests_total, _) = get_request_metrics();

    // Calculate approximate QPS and RPM
    // These are simplified approximations; real implementation would track
    // requests over sliding time windows
    let uptime_secs = std::time::Instant::now()
        .elapsed()
        .as_secs()
        .max(1);
    let qps = requests_total / uptime_secs;
    let rpm = requests_total.max(1) * 60 / uptime_secs.max(1);

    let timestamp = chrono::Utc::now().to_rfc3339();

    DashboardMetrics {
        msg_type: "metrics".to_string(),
        timestamp,
        traffic: TrafficMetrics {
            qps,
            rpm,
            active_ws_connections: ACTIVE_WS_CONNECTIONS.load(Ordering::Relaxed),
            in_flight: get_in_flight_requests(),
            latency_p50: 120, // Placeholder - would come from histogram percentile calculation
            latency_p99: 850, // Placeholder - would come from histogram percentile calculation
        },
        tokens: TokenMetrics {
            prompt_total,
            completion_total,
            rate_per_sec: (prompt_total + completion_total) / uptime_secs.max(1),
            cost_total_micro: cost_total_nano / 1000, // Convert nanodollars to microdollars
        },
        channels: ChannelMetrics {
            healthy: 0, // Placeholder - would query channel state
            unhealthy: 0, // Placeholder - would query channel state
            total: 0, // Placeholder - would query channel count from database
        },
        system: SystemResourceMetrics {
            cpu_usage_percent: system_metrics.as_ref().map(|m| m.cpu.usage_percent).unwrap_or(0.0),
            memory_usage_percent: system_metrics.as_ref().map(|m| m.memory.usage_percent).unwrap_or(0.0),
            memory_used_formatted: system_metrics
                .as_ref()
                .map(|m| m.memory.used_formatted())
                .unwrap_or_else(|| "N/A".to_string()),
            disk_usage_percent: system_metrics
                .as_ref()
                .map(|m| {
                    m.disks.iter()
                        .map(|d| d.usage_percent)
                        .sum::<f32>() / m.disks.len().max(1) as f32
                })
                .unwrap_or(0.0),
        },
    }
}

/// Get token metrics from Prometheus counters.
fn get_token_metrics() -> (u64, u64, u64) {
    let prompt_total = metrics::TOKENS_PROMPT_TOTAL.get();
    let completion_total = metrics::TOKENS_COMPLETION_TOTAL.get();
    let cost_total_nano = metrics::COST_TOTAL_NANO.get();

    (prompt_total as u64, completion_total as u64, cost_total_nano as u64)
}

/// Get request metrics from Prometheus counters.
fn get_request_metrics() -> (u64, u64) {
    // Get total requests from counter
    let metric_families = metrics::REGISTRY.gather();
    let mut requests_total = 0u64;
    let mut requests_error = 0u64;

    for family in &metric_families {
        if family.get_name() == "burncloud_requests_total" {
            for metric in family.get_metric() {
                let value = metric.get_counter().get_value() as u64;
                for label in metric.get_label() {
                    if label.get_name() == "status" {
                        if label.get_value() == "success" {
                            requests_total += value;
                        } else {
                            requests_error += value;
                        }
                    }
                }
            }
        }
    }

    (requests_total, requests_error)
}

/// Get in-flight requests from Prometheus gauge.
fn get_in_flight_requests() -> u64 {
    let metric_families = metrics::REGISTRY.gather();
    let mut in_flight = 0u64;

    for family in &metric_families {
        if family.get_name() == "burncloud_requests_in_flight" {
            for metric in family.get_metric() {
                in_flight += metric.get_gauge().get_value() as u64;
            }
        }
    }

    in_flight
}

/// Get number of active WebSocket connections.
pub fn get_active_ws_connections() -> u64 {
    ACTIVE_WS_CONNECTIONS.load(Ordering::Relaxed)
}

/// Broadcast metrics update to all connected WebSocket clients.
pub fn broadcast_metrics(metrics: DashboardMetrics) {
    let _ = get_metrics_channel().send(metrics);
}

/// Start background metrics collection task.
pub fn start_metrics_collection(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        
        loop {
            interval.tick().await;
            
            // Update system metrics in Prometheus
            metrics::update_system_metrics();
            
            // Collect and broadcast dashboard metrics
            let dashboard_metrics = collect_dashboard_metrics(&state).await;
            broadcast_metrics(dashboard_metrics);
        }
    });
}
