//! Webhook notification channel

use super::NotificationChannel;
use crate::types::{Alert, AlertError};
use async_trait::async_trait;
use serde_json::json;

/// Webhook notification channel
pub struct WebhookChannel {
    url: Option<String>,
    client: reqwest::Client,
}

impl WebhookChannel {
    /// Create a new webhook channel
    pub fn new(url: Option<String>) -> Self {
        Self {
            url,
            client: reqwest::Client::new(),
        }
    }

    /// Create from environment variable
    pub fn from_env() -> Self {
        Self::new(std::env::var("ALERT_WEBHOOK_URL").ok())
    }
}

#[async_trait]
impl NotificationChannel for WebhookChannel {
    async fn send(&self, alert: &Alert) -> Result<(), AlertError> {
        let url = self.url.as_ref().ok_or_else(|| {
            AlertError::ChannelUnavailable("Webhook URL not configured".to_string())
        })?;

        let payload = json!({
            "id": alert.id,
            "type": alert.alert_type,
            "level": alert.level,
            "status": alert.status,
            "message": alert.message,
            "triggered_at": alert.triggered_at.to_rfc3339(),
        });

        let response = self.client.post(url).json(&payload).send().await?;

        if !response.status().is_success() {
            return Err(AlertError::NotificationFailed(format!(
                "Webhook returned status: {}",
                response.status()
            )));
        }

        log::info!("Alert {} sent via webhook to {}", alert.id, url);
        Ok(())
    }

    fn name(&self) -> &'static str {
        "webhook"
    }

    fn is_configured(&self) -> bool {
        self.url.is_some()
    }
}
