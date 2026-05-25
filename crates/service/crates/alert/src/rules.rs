//! Alert rule evaluation

use crate::types::{Alert, AlertRule, AlertStatus, AlertType};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

/// Alert rule evaluator
pub struct AlertRuleEvaluator {
    /// Active alerts (for silence period tracking)
    active_alerts: HashMap<String, Alert>,
    /// Configured rules
    rules: Vec<AlertRule>,
}

impl AlertRuleEvaluator {
    /// Create a new rule evaluator
    pub fn new(rules: Vec<AlertRule>) -> Self {
        Self {
            active_alerts: HashMap::new(),
            rules,
        }
    }

    /// Add a new rule
    pub fn add_rule(&mut self, rule: AlertRule) {
        log::info!("Rule '{}' added for alert type {:?}", rule.name, rule.alert_type);
        self.rules.push(rule);
    }

    /// Get a key for the alert type (based on discriminant, not values)
    fn get_alert_key(alert_type: &AlertType) -> String {
        // Use discriminant to group alerts of the same type together
        // This ensures MemoryHigh alerts are grouped regardless of the percentage value
        match alert_type {
            AlertType::ChannelFailure { channel_id, .. } => format!("ChannelFailure:{}", channel_id),
            AlertType::ChannelHighLatency { channel_id, .. } => format!("ChannelHighLatency:{}", channel_id),
            AlertType::ChannelQuotaLow { channel_id, .. } => format!("ChannelQuotaLow:{}", channel_id),
            AlertType::SystemRestart => "SystemRestart".to_string(),
            AlertType::MemoryHigh { .. } => "MemoryHigh".to_string(),
            AlertType::QueueBacklog { .. } => "QueueBacklog".to_string(),
            AlertType::UserQuotaExhausted { user_id } => format!("UserQuotaExhausted:{}", user_id),
            AlertType::AbnormalTraffic { .. } => "AbnormalTraffic".to_string(),
            AlertType::CostAnomaly { .. } => "CostAnomaly".to_string(),
            AlertType::Custom { name } => format!("Custom:{}", name),
        }
    }

    /// Evaluate a condition and potentially trigger an alert
    pub fn evaluate(&mut self, alert_type: &AlertType, current_value: u64) -> Option<Alert> {
        // Find matching rule (by discriminant)
        let rule = self
            .rules
            .iter()
            .find(|r| std::mem::discriminant(&r.alert_type) == std::mem::discriminant(alert_type) && r.enabled)?;

        // Check if threshold is exceeded
        if current_value < rule.threshold {
            return None;
        }

        // Generate alert key for silence period check
        let alert_key = Self::get_alert_key(alert_type);

        // Check silence period
        if let Some(existing) = self.active_alerts.get(&alert_key) {
            let elapsed = Utc::now().signed_duration_since(existing.triggered_at);
            if elapsed.num_seconds() < rule.silence_period.as_secs() as i64 {
                // Still in silence period
                log::debug!("Alert {} is in silence period, skipping", alert_key);
                return None;
            }
        }

        // Create new alert
        let alert = Alert {
            id: Uuid::new_v4().to_string(),
            alert_type: alert_type.clone(),
            level: rule.level,
            status: AlertStatus::Active,
            message: Self::generate_message(alert_type, current_value, rule.threshold),
            triggered_at: Utc::now(),
            resolved_at: None,
            trigger_count: self
                .active_alerts
                .get(&alert_key)
                .map(|a| a.trigger_count + 1)
                .unwrap_or(1),
        };

        // Store as active alert
        self.active_alerts.insert(alert_key, alert.clone());

        Some(alert)
    }

    /// Mark an alert as resolved
    pub fn resolve(&mut self, alert_type: &AlertType) -> Option<Alert> {
        let alert_key = Self::get_alert_key(alert_type);

        if let Some(mut alert) = self.active_alerts.remove(&alert_key) {
            alert.status = AlertStatus::Resolved;
            alert.resolved_at = Some(Utc::now());
            Some(alert)
        } else {
            None
        }
    }

    /// Get all active alerts
    pub fn get_active_alerts(&self) -> Vec<&Alert> {
        self.active_alerts.values().collect()
    }

    /// Generate alert message (sanitized - no sensitive data)
    fn generate_message(alert_type: &AlertType, current: u64, threshold: u64) -> String {
        match alert_type {
            AlertType::ChannelFailure { channel_name, .. } => {
                format!("Channel '{}' has failed {} times (threshold: {})", channel_name, current, threshold)
            }
            AlertType::ChannelHighLatency { latency_ms, .. } => {
                format!("Channel latency is {}ms (threshold: {}ms)", latency_ms, threshold)
            }
            AlertType::ChannelQuotaLow { remaining_percent, .. } => {
                format!("Channel quota is at {}% (threshold: {}%)", remaining_percent, threshold)
            }
            AlertType::SystemRestart => "System has restarted unexpectedly".to_string(),
            AlertType::MemoryHigh { usage_percent } => {
                format!("Memory usage is at {}% (threshold: {}%)", usage_percent, threshold)
            }
            AlertType::QueueBacklog { queue_size } => {
                format!("Request queue has {} items (threshold: {})", queue_size, threshold)
            }
            AlertType::UserQuotaExhausted { .. } => {
                format!("User quota exhausted (threshold: {})", threshold)
            }
            AlertType::AbnormalTraffic { requests_per_minute } => {
                format!(
                    "Abnormal traffic detected: {} requests/min (threshold: {})",
                    requests_per_minute, threshold
                )
            }
            AlertType::CostAnomaly { increase_percent } => {
                format!("Cost increased by {}% (threshold: {}%)", increase_percent, threshold)
            }
            AlertType::Custom { name } => {
                format!("Custom alert '{}' triggered: {} (threshold: {})", name, current, threshold)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AlertLevel;
    use std::time::Duration;

    #[test]
    fn test_rule_evaluation() {
        let rule = AlertRule {
            name: "channel_failure".to_string(),
            alert_type: AlertType::ChannelFailure {
                channel_id: "test".to_string(),
                channel_name: "Test Channel".to_string(),
            },
            threshold: 5,
            silence_period: Duration::from_secs(300),
            level: AlertLevel::Warning,
            enabled: true,
        };

        let mut evaluator = AlertRuleEvaluator::new(vec![rule]);

        // Below threshold - no alert
        let result = evaluator.evaluate(
            &AlertType::ChannelFailure {
                channel_id: "test".to_string(),
                channel_name: "Test Channel".to_string(),
            },
            3,
        );
        assert!(result.is_none());

        // At threshold - alert triggered
        let result = evaluator.evaluate(
            &AlertType::ChannelFailure {
                channel_id: "test".to_string(),
                channel_name: "Test Channel".to_string(),
            },
            5,
        );
        assert!(result.is_some());
        let alert = result.unwrap();
        assert_eq!(alert.level, AlertLevel::Warning);
        assert_eq!(alert.status, AlertStatus::Active);
    }

    #[test]
    fn test_silence_period() {
        let rule = AlertRule {
            name: "test".to_string(),
            alert_type: AlertType::MemoryHigh { usage_percent: 90 },
            threshold: 80,
            silence_period: Duration::from_secs(300),
            level: AlertLevel::Critical,
            enabled: true,
        };

        let mut evaluator = AlertRuleEvaluator::new(vec![rule]);

        // First alert
        let result = evaluator.evaluate(&AlertType::MemoryHigh { usage_percent: 85 }, 85);
        assert!(result.is_some());

        // Second alert within silence period - should be silenced
        // Note: using different usage_percent value but same alert type
        let result = evaluator.evaluate(&AlertType::MemoryHigh { usage_percent: 90 }, 90);
        assert!(result.is_none(), "Second alert should be silenced");
    }

    #[test]
    fn test_resolve_alert() {
        let rule = AlertRule {
            name: "test".to_string(),
            alert_type: AlertType::QueueBacklog { queue_size: 100 },
            threshold: 50,
            silence_period: Duration::from_secs(60),
            level: AlertLevel::Warning,
            enabled: true,
        };

        let mut evaluator = AlertRuleEvaluator::new(vec![rule]);

        // Trigger alert
        evaluator.evaluate(&AlertType::QueueBacklog { queue_size: 60 }, 60);

        // Resolve
        let resolved = evaluator.resolve(&AlertType::QueueBacklog { queue_size: 60 });
        assert!(resolved.is_some());
        let alert = resolved.unwrap();
        assert_eq!(alert.status, AlertStatus::Resolved);
        assert!(alert.resolved_at.is_some());
    }
}
