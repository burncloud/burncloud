//! Alert service for BurnCloud
//!
//! This module provides alert monitoring and notification capabilities for critical events.

pub mod channels;
pub mod rules;
pub mod service;
pub mod types;

// Re-export main public API
pub use channels::{
    EmailChannel, NotificationChannel, SlackChannel, TelegramChannel, WebhookChannel,
};
pub use rules::AlertRuleEvaluator;
pub use service::AlertService;
pub use types::{Alert, AlertConfig, AlertError, AlertLevel, AlertRule, AlertStatus, AlertType};
