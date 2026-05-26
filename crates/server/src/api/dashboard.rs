//! WebSocket dashboard handler for real-time metrics.
//!
//! Provides a WebSocket endpoint at `/ws/dashboard` that broadcasts
//! real-time metrics including traffic, tokens, channel health, and costs.

use crate::api::response::ok;
use crate::AppState;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
    response::Response,
    routing::get,
    Router,
};
use burncloud_service_monitor::DashboardService;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;

/// Dashboard WebSocket routes.
pub fn routes() -> Router<AppState> {
    Router::new().route("/ws/dashboard", get(dashboard_websocket_handler))
}

/// Handle WebSocket upgrade for dashboard.
async fn dashboard_websocket_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(|socket| handle_dashboard_socket(socket, state))
}

/// Handle the WebSocket connection for dashboard metrics.
async fn handle_dashboard_socket(socket: WebSocket, _state: AppState) {
    let (sender, mut receiver) = socket.split();

    // Wrap sender in Arc<Mutex> for sharing between tasks
    let sender = Arc::new(Mutex::new(sender));

    tracing::info!("Dashboard WebSocket client connected");

    // Send initial metrics immediately
    let initial_metrics = DashboardService::sample_metrics();
    let initial_json = match serde_json::to_string(&initial_metrics) {
        Ok(json) => json,
        Err(e) => {
            tracing::error!("Failed to serialize initial metrics: {}", e);
            return;
        }
    };

    {
        let mut sender_guard = sender.lock().await;
        if sender_guard
            .send(Message::Text(initial_json.into()))
            .await
            .is_err()
        {
            tracing::warn!("Failed to send initial metrics to client");
            return;
        }
    }

    // Spawn a task to periodically send metrics
    let sender_clone = sender.clone();
    let send_task = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(1));

        loop {
            ticker.tick().await;

            // Get current metrics
            // In production, this would pull from Prometheus metrics
            let metrics = DashboardService::sample_metrics();

            let json = match serde_json::to_string(&metrics) {
                Ok(json) => json,
                Err(e) => {
                    tracing::error!("Failed to serialize metrics: {}", e);
                    continue;
                }
            };

            let mut sender_guard = sender_clone.lock().await;
            if sender_guard.send(Message::Text(json.into())).await.is_err() {
                tracing::info!("Dashboard WebSocket client disconnected");
                break;
            }
        }
    });

    // Wait for client to close connection or error
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Close(_)) => {
                tracing::info!("Dashboard WebSocket client sent close frame");
                break;
            }
            Ok(Message::Ping(data)) => {
                // Respond to ping with pong
                let mut sender_guard = sender.lock().await;
                if sender_guard.send(Message::Pong(data)).await.is_err() {
                    break;
                }
            }
            Err(e) => {
                tracing::warn!("Dashboard WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    // Clean up the send task
    send_task.abort();

    tracing::info!("Dashboard WebSocket connection closed");
}

/// REST endpoint for dashboard metrics (for clients without WebSocket support).
pub async fn get_dashboard_metrics(State(_state): State<AppState>) -> impl IntoResponse {
    // In production, this would aggregate from Prometheus metrics
    let metrics = DashboardService::sample_metrics();
    ok(metrics).into_response()
}
