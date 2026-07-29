//! Email notification channel (stub implementation)
//!
//! # ⚠️ STUB IMPLEMENTATION
//! This channel is currently a stub. The `send` method only logs the alert
//! message without actually sending an email.
//! Do NOT rely on this channel for production email alerts until the actual
//! sending logic is implemented.

use super::NotificationChannel;
use crate::types::{Alert, AlertError};
use async_trait::async_trait;

/// Email notification channel
///
/// # ⚠️ STUB: This channel does NOT send actual emails.
/// It only logs the alert. Real email sending is not yet implemented.
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

        // ⚠️ STUB: Actual email sending is NOT implemented.
        // This channel only logs alerts. Replace with lettre or similar library.
        tracing::warn!(
            "Email channel is a STUB - alert only logged, not sent via email. \
             Alert: [{}] {} - {}",
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
