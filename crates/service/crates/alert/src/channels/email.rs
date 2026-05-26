//! Email notification channel (stub implementation)

use super::NotificationChannel;
use crate::types::{Alert, AlertError};
use async_trait::async_trait;

/// Email notification channel
pub struct EmailChannel {
    smtp_config: Option<String>,
}

impl EmailChannel {
    /// Create a new email channel
    pub fn new(smtp_config: Option<String>) -> Self {
        Self { smtp_config }
    }

    /// Create from environment variable
    pub fn from_env() -> Self {
        Self::new(std::env::var("ALERT_EMAIL_SMTP").ok())
    }
}

#[async_trait]
impl NotificationChannel for EmailChannel {
    async fn send(&self, alert: &Alert) -> Result<(), AlertError> {
        if !self.is_configured() {
            return Err(AlertError::ChannelUnavailable(
                "Email SMTP not configured".to_string(),
            ));
        }

        // TODO: Implement actual email sending using lettre or similar
        // For now, just log the alert
        log::info!(
            "Email alert: [{}] {} - {}",
            alert.level,
            alert.alert_type,
            alert.message
        );

        Ok(())
    }

    fn name(&self) -> &'static str {
        "email"
    }

    fn is_configured(&self) -> bool {
        self.smtp_config.is_some()
    }
}
