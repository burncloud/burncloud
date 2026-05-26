//! Alert service implementation

use crate::channels::{
    EmailChannel, NotificationChannel, SlackChannel, TelegramChannel, WebhookChannel,
};
use crate::rules::AlertRuleEvaluator;
use crate::types::{Alert, AlertConfig, AlertError, AlertRule, AlertType};

/// Alert service for monitoring and notifying critical events
pub struct AlertService {
    /// Rule evaluator
    evaluator: std::sync::Arc<tokio::sync::RwLock<AlertRuleEvaluator>>,
    /// Notification channels
    channels: Vec<Box<dyn NotificationChannel>>,
    /// Configuration (kept for future use)
    #[allow(dead_code)]
    config: AlertConfig,
}

impl AlertService {
    /// Create a new alert service with default configuration
    pub fn new() -> Self {
        Self::with_config(AlertConfig::default())
    }

    /// Create a new alert service with custom configuration
    pub fn with_config(config: AlertConfig) -> Self {
        let rules = config.rules.clone();
        let mut channels: Vec<Box<dyn NotificationChannel>> = Vec::new();

        // Add configured channels
        if config.webhook_url.is_some() {
            channels.push(Box::new(WebhookChannel::new(config.webhook_url.clone())));
        }
        if config.email_smtp.is_some() {
            channels.push(Box::new(EmailChannel::new(config.email_smtp.clone())));
        }
        if config.telegram_bot_token.is_some() && config.telegram_chat_id.is_some() {
            channels.push(Box::new(TelegramChannel::new(
                config.telegram_bot_token.clone(),
                config.telegram_chat_id.clone(),
            )));
        }
        if config.slack_webhook.is_some() {
            channels.push(Box::new(SlackChannel::new(config.slack_webhook.clone())));
        }

        Self {
            evaluator: std::sync::Arc::new(tokio::sync::RwLock::new(AlertRuleEvaluator::new(
                rules,
            ))),
            channels,
            config,
        }
    }

    /// Check a condition and potentially trigger an alert
    pub async fn check(&self, alert_type: &AlertType, value: u64) -> Option<Alert> {
        let mut evaluator = self.evaluator.write().await;
        evaluator.evaluate(alert_type, value)
    }

    /// Send alert notifications through all configured channels
    pub async fn notify(&self, alert: &Alert) -> Result<(), AlertError> {
        if self.channels.is_empty() {
            log::warn!("No notification channels configured, alert not sent");
            return Ok(());
        }

        let mut errors = Vec::new();

        for channel in &self.channels {
            if !channel.is_configured() {
                continue;
            }

            // Retry logic: 3 attempts
            let mut attempts = 0;
            loop {
                match channel.send(alert).await {
                    Ok(()) => break,
                    Err(e) => {
                        attempts += 1;
                        if attempts >= 3 {
                            log::error!(
                                "Failed to send alert via {} after 3 attempts: {}",
                                channel.name(),
                                e
                            );
                            errors.push(format!("{}: {}", channel.name(), e));
                            break;
                        }
                        // Wait before retry
                        tokio::time::sleep(std::time::Duration::from_millis(100 * attempts as u64))
                            .await;
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(AlertError::NotificationFailed(errors.join("; ")))
        }
    }

    /// Check and notify in one operation
    pub async fn check_and_notify(
        &self,
        alert_type: &AlertType,
        value: u64,
    ) -> Option<Result<Alert, AlertError>> {
        let alert = self.check(alert_type, value).await?;
        let result = self.notify(&alert).await.map(|_| alert.clone());
        Some(result)
    }

    /// Resolve an alert and send recovery notification
    pub async fn resolve(&self, alert_type: &AlertType) -> Option<Alert> {
        let mut evaluator = self.evaluator.write().await;
        let alert = evaluator.resolve(alert_type)?;

        // Send recovery notification
        if let Err(e) = self.notify(&alert).await {
            log::error!("Failed to send recovery notification: {}", e);
        }

        Some(alert)
    }

    /// Get all active alerts
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let evaluator = self.evaluator.read().await;
        evaluator.get_active_alerts().into_iter().cloned().collect()
    }

    /// Add a new rule
    pub async fn add_rule(&self, rule: AlertRule) {
        let mut evaluator = self.evaluator.write().await;
        evaluator.add_rule(rule);
    }

    /// Report channel failure
    pub async fn report_channel_failure(
        &self,
        channel_id: &str,
        channel_name: &str,
        failure_count: u64,
    ) {
        let alert_type = AlertType::ChannelFailure {
            channel_id: channel_id.to_string(),
            channel_name: channel_name.to_string(),
        };

        if let Some(Err(e)) = self.check_and_notify(&alert_type, failure_count).await {
            log::error!("Failed to send channel failure alert: {}", e);
        }
    }

    /// Report channel recovery
    pub async fn report_channel_recovery(&self, channel_id: &str, channel_name: &str) {
        let alert_type = AlertType::ChannelFailure {
            channel_id: channel_id.to_string(),
            channel_name: channel_name.to_string(),
        };

        if let Some(_alert) = self.resolve(&alert_type).await {
            log::info!("Channel '{}' recovered, alert resolved", channel_name);
        }
    }

    /// Report high memory usage
    pub async fn report_memory_high(&self, usage_percent: u8) {
        let alert_type = AlertType::MemoryHigh { usage_percent };
        if let Some(Err(e)) = self
            .check_and_notify(&alert_type, usage_percent as u64)
            .await
        {
            log::error!("Failed to send memory alert: {}", e);
        }
    }

    /// Report queue backlog
    pub async fn report_queue_backlog(&self, queue_size: u64) {
        let alert_type = AlertType::QueueBacklog { queue_size };
        if let Some(Err(e)) = self.check_and_notify(&alert_type, queue_size).await {
            log::error!("Failed to send queue backlog alert: {}", e);
        }
    }
}

impl Default for AlertService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AlertLevel;
    use std::time::Duration;

    #[tokio::test]
    async fn test_alert_service_creation() {
        let service = AlertService::new();
        assert!(service.channels.is_empty() || service.channels.iter().any(|c| !c.is_configured()));
    }

    #[tokio::test]
    async fn test_check_below_threshold() {
        let mut config = AlertConfig::default();
        config.rules.push(AlertRule {
            name: "test".to_string(),
            alert_type: AlertType::QueueBacklog { queue_size: 100 },
            threshold: 50,
            silence_period: Duration::from_secs(60),
            level: AlertLevel::Warning,
            enabled: true,
        });

        let service = AlertService::with_config(config);

        // Below threshold - no alert
        let result = service
            .check(&AlertType::QueueBacklog { queue_size: 30 }, 30)
            .await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_check_above_threshold() {
        let mut config = AlertConfig::default();
        config.rules.push(AlertRule {
            name: "test".to_string(),
            alert_type: AlertType::QueueBacklog { queue_size: 100 },
            threshold: 50,
            silence_period: Duration::from_secs(60),
            level: AlertLevel::Warning,
            enabled: true,
        });

        let service = AlertService::with_config(config);

        // Above threshold - alert triggered
        let result = service
            .check(&AlertType::QueueBacklog { queue_size: 60 }, 60)
            .await;
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_get_active_alerts() {
        let mut config = AlertConfig::default();
        config.rules.push(AlertRule {
            name: "test".to_string(),
            alert_type: AlertType::MemoryHigh { usage_percent: 90 },
            threshold: 80,
            silence_period: Duration::from_secs(60),
            level: AlertLevel::Critical,
            enabled: true,
        });

        let service = AlertService::with_config(config);

        // Initially no active alerts
        let alerts = service.get_active_alerts().await;
        assert!(alerts.is_empty());

        // Trigger alert
        service
            .check(&AlertType::MemoryHigh { usage_percent: 85 }, 85)
            .await;

        // Now should have one active alert
        let alerts = service.get_active_alerts().await;
        assert_eq!(alerts.len(), 1);
    }
}
