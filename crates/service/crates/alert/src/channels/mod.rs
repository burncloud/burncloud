//! Notification channels for alert delivery

mod email;
mod slack;
mod telegram;
mod webhook;

pub use email::EmailChannel;
pub use slack::SlackChannel;
pub use telegram::TelegramChannel;
pub use webhook::WebhookChannel;

use crate::types::{Alert, AlertError};
use async_trait::async_trait;

/// Trait for notification channels
#[async_trait]
pub trait NotificationChannel: Send + Sync {
    /// Send an alert notification through this channel
    async fn send(&self, alert: &Alert) -> Result<(), AlertError>;

    /// Channel name for logging
    fn name(&self) -> &'static str;

    /// Check if channel is configured and available
    fn is_configured(&self) -> bool;
}
