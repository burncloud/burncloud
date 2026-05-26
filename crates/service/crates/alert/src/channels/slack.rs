//! Slack notification channel

use super::NotificationChannel;
use crate::types::{Alert, AlertError, AlertLevel};
use async_trait::async_trait;
use serde_json::json;

/// Slack notification channel
pub struct SlackChannel {
    webhook_url: Option<String>,
    client: reqwest::Client,
}

impl SlackChannel {
    /// Create a new Slack channel
    pub fn new(webhook_url: Option<String>) -> Self {
        Self {
            webhook_url,
            client: reqwest::Client::new(),
        }
    }

    /// Create from environment variable
    pub fn from_env() -> Self {
        Self::new(std::env::var("ALERT_SLACK_WEBHOOK").ok())
    }
}

#[async_trait]
impl NotificationChannel for SlackChannel {
    async fn send(&self, alert: &Alert) -> Result<(), AlertError> {
        let webhook_url = self.webhook_url.as_ref().ok_or_else(|| {
            AlertError::ChannelUnavailable("Slack webhook not configured".to_string())
        })?;

        let color = match alert.level {
            AlertLevel::Info => "#36a64f",
            AlertLevel::Warning => "#ffcc00",
            AlertLevel::Critical => "#ff0000",
        };

        let payload = json!({
            "attachments": [{
                "color": color,
                "title": format!("{} Alert", alert.level),
                "text": alert.message,
                "fields": [{
                    "title": "Type",
                    "value": format!("{}", alert.alert_type),
                    "short": true
                }, {
                    "title": "Status",
                    "value": format!("{}", alert.status),
                    "short": true
                }],
                "footer": "BurnCloud Alert System",
                "ts": alert.triggered_at.timestamp(),
            }]
        });

        let response = self.client.post(webhook_url).json(&payload).send().await?;

        if !response.status().is_success() {
            return Err(AlertError::NotificationFailed(format!(
                "Slack webhook returned status: {}",
                response.status()
            )));
        }

        log::info!("Alert {} sent via Slack", alert.id);
        Ok(())
    }

    fn name(&self) -> &'static str {
        "slack"
    }

    fn is_configured(&self) -> bool {
        self.webhook_url.is_some()
    }
}
