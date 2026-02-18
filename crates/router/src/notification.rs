//! Notification Module
//!
//! This module provides notification services for:
//! - New model discovery alerts
//! - Price missing alerts
//! - Channel error alerts
//!
//! Notifications can be sent via various channels (webhook, email, etc.)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Notification type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    /// New model discovered for the first time
    NewModel,
    /// Price configuration missing for a model
    PriceMissing,
    /// Channel encountered an error
    ChannelError,
    /// Channel authentication failed
    AuthFailed,
    /// Channel balance exhausted
    BalanceExhausted,
    /// API version deprecated
    ApiVersionDeprecated,
}

/// Notification priority levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// A notification message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Type of notification
    pub notification_type: NotificationType,
    /// Priority level
    pub priority: Priority,
    /// Notification title
    pub title: String,
    /// Detailed message
    pub message: String,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Timestamp of the notification
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Notification {
    /// Create a new notification
    pub fn new(
        notification_type: NotificationType,
        priority: Priority,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            notification_type,
            priority,
            title: title.into(),
            message: message.into(),
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Add metadata to the notification
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Notification channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Webhook URL for notifications (optional)
    pub webhook_url: Option<String>,
    /// Email recipients (optional)
    #[serde(default)]
    pub email_recipients: Vec<String>,
    /// Enable/disable notifications
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Minimum priority level to send notifications
    #[serde(default = "default_min_priority")]
    pub min_priority: Priority,
    /// Rate limit for notifications (per hour)
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_hour: u32,
}

fn default_enabled() -> bool {
    true
}

fn default_min_priority() -> Priority {
    Priority::Medium
}

fn default_rate_limit() -> u32 {
    10
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            webhook_url: None,
            email_recipients: Vec::new(),
            enabled: true,
            min_priority: default_min_priority(),
            rate_limit_per_hour: default_rate_limit(),
        }
    }
}

/// Notification service for sending alerts
pub struct NotificationService {
    /// Configuration for the notification service
    config: NotificationConfig,
    /// HTTP client for webhook calls
    http_client: reqwest::Client,
    /// Rate limiter state (count per notification type)
    rate_limiter: std::collections::HashMap<String, (u32, std::time::Instant)>,
}

impl NotificationService {
    /// Create a new notification service
    pub fn new(config: NotificationConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
            rate_limiter: std::collections::HashMap::new(),
        }
    }

    /// Create a notification service with default configuration
    pub fn with_defaults() -> Self {
        Self::new(NotificationConfig::default())
    }

    /// Check if notifications are enabled and should be sent
    fn should_send(&mut self, notification: &Notification) -> bool {
        if !self.config.enabled {
            return false;
        }

        // Check priority threshold
        let priority_level = match notification.priority {
            Priority::Low => 0,
            Priority::Medium => 1,
            Priority::High => 2,
            Priority::Critical => 3,
        };

        let min_priority_level = match self.config.min_priority {
            Priority::Low => 0,
            Priority::Medium => 1,
            Priority::High => 2,
            Priority::Critical => 3,
        };

        if priority_level < min_priority_level {
            return false;
        }

        // Check rate limit
        let key = format!("{:?}", notification.notification_type);
        let now = std::time::Instant::now();
        let hour_ago = std::time::Duration::from_secs(3600);

        if let Some((count, last_reset)) = self.rate_limiter.get_mut(&key) {
            if now.duration_since(*last_reset) > hour_ago {
                // Reset counter
                *count = 1;
                *last_reset = now;
            } else if *count >= self.config.rate_limit_per_hour {
                return false;
            } else {
                *count += 1;
            }
        } else {
            self.rate_limiter.insert(key, (1, now));
        }

        true
    }

    /// Send a notification
    pub async fn send(&mut self, notification: Notification) -> anyhow::Result<()> {
        if !self.should_send(&notification) {
            return Ok(());
        }

        // Log the notification
        println!(
            "[Notification] [{:?}] {:?}: {} - {}",
            notification.priority,
            notification.notification_type,
            notification.title,
            notification.message
        );

        // Send via webhook if configured
        if let Some(ref webhook_url) = self.config.webhook_url {
            self.send_webhook(webhook_url, &notification).await?;
        }

        // TODO: Implement email sending when email_recipients is configured

        Ok(())
    }

    /// Send notification via webhook
    async fn send_webhook(&self, url: &str, notification: &Notification) -> anyhow::Result<()> {
        let response = self
            .http_client
            .post(url)
            .json(notification)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Webhook notification failed: {} - {}", status, body);
        }

        Ok(())
    }

    /// Notify about a new model discovery
    pub async fn notify_new_model(
        &mut self,
        model: &str,
        channel_id: i32,
        channel_name: &str,
    ) -> anyhow::Result<()> {
        let notification = Notification::new(
            NotificationType::NewModel,
            Priority::Medium,
            format!("New Model Discovered: {}", model),
            format!(
                "A new model '{}' was discovered on channel '{}' (ID: {}). \
                 Please configure pricing if not already set.",
                model, channel_name, channel_id
            ),
        )
        .with_metadata("model", model)
        .with_metadata("channel_id", channel_id.to_string())
        .with_metadata("channel_name", channel_name);

        self.send(notification).await
    }

    /// Notify about missing price configuration
    pub async fn notify_price_missing(&mut self, model: &str) -> anyhow::Result<()> {
        let notification = Notification::new(
            NotificationType::PriceMissing,
            Priority::High,
            format!("Price Missing for Model: {}", model),
            format!(
                "Model '{}' is being requested but has no price configuration. \
                 Please add pricing information to enable quota tracking.",
                model
            ),
        )
        .with_metadata("model", model);

        self.send(notification).await
    }

    /// Notify about a channel error
    pub async fn notify_channel_error(
        &mut self,
        channel_id: i32,
        channel_name: &str,
        error_type: &str,
        error_message: &str,
    ) -> anyhow::Result<()> {
        let priority = match error_type {
            "AuthFailed" | "BalanceExhausted" => Priority::Critical,
            "RateLimited" => Priority::Medium,
            _ => Priority::High,
        };

        let notification = Notification::new(
            NotificationType::ChannelError,
            priority,
            format!("Channel Error: {} - {}", channel_name, error_type),
            format!(
                "Channel '{}' (ID: {}) encountered an error: {} - {}",
                channel_name, channel_id, error_type, error_message
            ),
        )
        .with_metadata("channel_id", channel_id.to_string())
        .with_metadata("channel_name", channel_name)
        .with_metadata("error_type", error_type);

        self.send(notification).await
    }

    /// Notify about authentication failure
    pub async fn notify_auth_failed(
        &mut self,
        channel_id: i32,
        channel_name: &str,
        error_message: &str,
    ) -> anyhow::Result<()> {
        let notification = Notification::new(
            NotificationType::AuthFailed,
            Priority::Critical,
            format!("Authentication Failed: {}", channel_name),
            format!(
                "Channel '{}' (ID: {}) authentication failed: {}. \
                 The channel has been disabled until the API key is updated.",
                channel_name, channel_id, error_message
            ),
        )
        .with_metadata("channel_id", channel_id.to_string())
        .with_metadata("channel_name", channel_name);

        self.send(notification).await
    }

    /// Notify about balance exhaustion
    pub async fn notify_balance_exhausted(
        &mut self,
        channel_id: i32,
        channel_name: &str,
    ) -> anyhow::Result<()> {
        let notification = Notification::new(
            NotificationType::BalanceExhausted,
            Priority::Critical,
            format!("Balance Exhausted: {}", channel_name),
            format!(
                "Channel '{}' (ID: {}) has exhausted its balance. \
                 Please add credits to restore service.",
                channel_name, channel_id
            ),
        )
        .with_metadata("channel_id", channel_id.to_string())
        .with_metadata("channel_name", channel_name);

        self.send(notification).await
    }

    /// Notify about API version deprecation
    pub async fn notify_api_version_deprecated(
        &mut self,
        channel_id: i32,
        channel_name: &str,
        old_version: &str,
        new_version: &str,
    ) -> anyhow::Result<()> {
        let notification = Notification::new(
            NotificationType::ApiVersionDeprecated,
            Priority::High,
            format!("API Version Deprecated: {}", channel_name),
            format!(
                "Channel '{}' (ID: {}) is using deprecated API version '{}'. \
                 New version '{}' is available. Consider updating the channel configuration.",
                channel_name, channel_id, old_version, new_version
            ),
        )
        .with_metadata("channel_id", channel_id.to_string())
        .with_metadata("channel_name", channel_name)
        .with_metadata("old_version", old_version)
        .with_metadata("new_version", new_version);

        self.send(notification).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_creation() {
        let notification = Notification::new(
            NotificationType::NewModel,
            Priority::Medium,
            "Test Title",
            "Test Message",
        );

        assert_eq!(notification.notification_type, NotificationType::NewModel);
        assert_eq!(notification.priority, Priority::Medium);
        assert_eq!(notification.title, "Test Title");
        assert_eq!(notification.message, "Test Message");
        assert!(notification.metadata.is_empty());
    }

    #[test]
    fn test_notification_with_metadata() {
        let notification = Notification::new(
            NotificationType::ChannelError,
            Priority::High,
            "Error",
            "Something went wrong",
        )
        .with_metadata("channel_id", "123")
        .with_metadata("error_code", "500");

        assert_eq!(notification.metadata.get("channel_id"), Some(&"123".to_string()));
        assert_eq!(notification.metadata.get("error_code"), Some(&"500".to_string()));
    }

    #[test]
    fn test_notification_config_default() {
        let config = NotificationConfig::default();

        assert!(config.enabled);
        assert!(config.webhook_url.is_none());
        assert!(config.email_recipients.is_empty());
        assert_eq!(config.min_priority, Priority::Medium);
        assert_eq!(config.rate_limit_per_hour, 10);
    }

    #[test]
    fn test_notification_service_creation() {
        let service = NotificationService::with_defaults();

        assert!(service.config.enabled);
    }

    #[tokio::test]
    async fn test_notification_service_disabled() {
        let config = NotificationConfig {
            enabled: false,
            ..Default::default()
        };
        let mut service = NotificationService::new(config);

        let notification = Notification::new(
            NotificationType::NewModel,
            Priority::Critical,
            "Test",
            "Test",
        );

        // Should not send and should succeed silently
        let result = service.send(notification).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_should_send_priority_filter() {
        let config = NotificationConfig {
            enabled: true,
            min_priority: Priority::High,
            ..Default::default()
        };
        let mut service = NotificationService::new(config);

        // Low priority should be filtered
        let low = Notification::new(NotificationType::NewModel, Priority::Low, "", "");
        assert!(!service.should_send(&low));

        // Medium priority should be filtered
        let medium = Notification::new(NotificationType::NewModel, Priority::Medium, "", "");
        assert!(!service.should_send(&medium));

        // High priority should pass
        let high = Notification::new(NotificationType::NewModel, Priority::High, "", "");
        assert!(service.should_send(&high));

        // Critical priority should pass
        let critical = Notification::new(NotificationType::NewModel, Priority::Critical, "", "");
        assert!(service.should_send(&critical));
    }

    #[test]
    fn test_notification_serialization() {
        let notification = Notification::new(
            NotificationType::ChannelError,
            Priority::High,
            "Test",
            "Test message",
        );

        let json = serde_json::to_string(&notification).unwrap();
        assert!(json.contains("channel_error"));
        assert!(json.contains("high"));
        assert!(json.contains("Test"));
    }
}
