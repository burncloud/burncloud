//! Alert types and configurations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertLevel {
    /// Informational - no immediate action required
    Info,
    /// Warning - attention needed soon
    Warning,
    /// Critical - immediate action required
    Critical,
}

impl fmt::Display for AlertLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlertLevel::Info => write!(f, "Info"),
            AlertLevel::Warning => write!(f, "Warning"),
            AlertLevel::Critical => write!(f, "Critical"),
        }
    }
}

/// Alert type classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    // Channel health alerts
    ChannelFailure {
        channel_id: String,
        channel_name: String,
    },
    ChannelHighLatency {
        channel_id: String,
        latency_ms: u64,
    },
    ChannelQuotaLow {
        channel_id: String,
        remaining_percent: u8,
    },

    // System alerts
    SystemRestart,
    MemoryHigh {
        usage_percent: u8,
    },
    QueueBacklog {
        queue_size: u64,
    },

    // Business alerts
    UserQuotaExhausted {
        user_id: String,
    },
    AbnormalTraffic {
        requests_per_minute: u64,
    },
    CostAnomaly {
        increase_percent: u8,
    },

    // Custom alert
    Custom {
        name: String,
    },
}

impl fmt::Display for AlertType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlertType::ChannelFailure { channel_name, .. } => {
                write!(f, "ChannelFailure({})", channel_name)
            }
            AlertType::ChannelHighLatency { latency_ms, .. } => {
                write!(f, "ChannelHighLatency({}ms)", latency_ms)
            }
            AlertType::ChannelQuotaLow {
                remaining_percent, ..
            } => write!(f, "ChannelQuotaLow({}%)", remaining_percent),
            AlertType::SystemRestart => write!(f, "SystemRestart"),
            AlertType::MemoryHigh { usage_percent } => write!(f, "MemoryHigh({}%)", usage_percent),
            AlertType::QueueBacklog { queue_size } => write!(f, "QueueBacklog({})", queue_size),
            AlertType::UserQuotaExhausted { .. } => write!(f, "UserQuotaExhausted"),
            AlertType::AbnormalTraffic {
                requests_per_minute,
            } => write!(f, "AbnormalTraffic({}/min)", requests_per_minute),
            AlertType::CostAnomaly { increase_percent } => {
                write!(f, "CostAnomaly(+{}%)", increase_percent)
            }
            AlertType::Custom { name } => write!(f, "Custom({})", name),
        }
    }
}

/// Alert status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertStatus {
    /// Alert is active
    Active,
    /// Alert is silenced (within silence period)
    Silenced,
    /// Alert has been resolved
    Resolved,
}

impl fmt::Display for AlertStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlertStatus::Active => write!(f, "Active"),
            AlertStatus::Silenced => write!(f, "Silenced"),
            AlertStatus::Resolved => write!(f, "Resolved"),
        }
    }
}

/// Alert record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Unique alert ID
    pub id: String,
    /// Alert type
    pub alert_type: AlertType,
    /// Severity level
    pub level: AlertLevel,
    /// Current status
    pub status: AlertStatus,
    /// Alert message (sanitized - no sensitive data)
    pub message: String,
    /// Timestamp when alert was triggered
    pub triggered_at: DateTime<Utc>,
    /// Timestamp when alert was resolved (if applicable)
    pub resolved_at: Option<DateTime<Utc>>,
    /// Number of times this alert has been triggered
    pub trigger_count: u32,
}

/// Alert rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule name
    pub name: String,
    /// Alert type this rule monitors
    pub alert_type: AlertType,
    /// Threshold for triggering
    pub threshold: u64,
    /// Silence period after alert (prevents duplicate notifications)
    #[serde(with = "duration_serde")]
    pub silence_period: Duration,
    /// Alert level when triggered
    pub level: AlertLevel,
    /// Whether this rule is enabled
    pub enabled: bool,
}

/// Global alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Webhook URL for notifications
    pub webhook_url: Option<String>,
    /// Email configuration
    pub email_smtp: Option<String>,
    /// Telegram bot token
    pub telegram_bot_token: Option<String>,
    /// Telegram chat ID
    pub telegram_chat_id: Option<String>,
    /// Slack webhook URL
    pub slack_webhook: Option<String>,
    /// Alert rules
    pub rules: Vec<AlertRule>,
    /// Default silence period
    #[serde(with = "duration_serde")]
    pub default_silence_period: Duration,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            webhook_url: std::env::var("ALERT_WEBHOOK_URL").ok(),
            email_smtp: std::env::var("ALERT_EMAIL_SMTP").ok(),
            telegram_bot_token: std::env::var("ALERT_TELEGRAM_BOT_TOKEN").ok(),
            telegram_chat_id: std::env::var("ALERT_TELEGRAM_CHAT_ID").ok(),
            slack_webhook: std::env::var("ALERT_SLACK_WEBHOOK").ok(),
            rules: Vec::new(),
            default_silence_period: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Alert error type
#[derive(Debug, thiserror::Error)]
pub enum AlertError {
    #[error("Failed to send notification: {0}")]
    NotificationFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Rule evaluation failed: {0}")]
    RuleEvaluationFailed(String),

    #[error("Channel unavailable: {0}")]
    ChannelUnavailable(String),

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
}

/// Custom serialization for Duration
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let secs = duration.as_secs();
        secs.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}
