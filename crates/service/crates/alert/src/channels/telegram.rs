//! Telegram notification channel

use super::NotificationChannel;
use crate::types::{Alert, AlertError, AlertLevel};
use async_trait::async_trait;
use serde_json::json;

/// Telegram notification channel
pub struct TelegramChannel {
    bot_token: Option<String>,
    chat_id: Option<String>,
    client: reqwest::Client,
}

impl TelegramChannel {
    /// Create a new Telegram channel
    pub fn new(bot_token: Option<String>, chat_id: Option<String>) -> Self {
        Self {
            bot_token,
            chat_id,
            client: reqwest::Client::new(),
        }
    }

    /// Create from environment variables
    pub fn from_env() -> Self {
        Self::new(
            std::env::var("ALERT_TELEGRAM_BOT_TOKEN").ok(),
            std::env::var("ALERT_TELEGRAM_CHAT_ID").ok(),
        )
    }
}

#[async_trait]
impl NotificationChannel for TelegramChannel {
    async fn send(&self, alert: &Alert) -> Result<(), AlertError> {
        let bot_token = self.bot_token.as_ref().ok_or_else(|| {
            AlertError::ChannelUnavailable("Telegram bot token not configured".to_string())
        })?;
        let chat_id = self.chat_id.as_ref().ok_or_else(|| {
            AlertError::ChannelUnavailable("Telegram chat ID not configured".to_string())
        })?;

        let level_emoji = match alert.level {
            AlertLevel::Info => "ℹ️",
            AlertLevel::Warning => "⚠️",
            AlertLevel::Critical => "🚨",
        };

        let message = format!(
            "{} *{} Alert*\n\n{}\n\nTime: {}",
            level_emoji,
            alert.level,
            alert.message,
            alert.triggered_at.format("%Y-%m-%d %H:%M:%S UTC")
        );

        let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

        let payload = json!({
            "chat_id": chat_id,
            "text": message,
            "parse_mode": "Markdown",
        });

        let response = self.client.post(&url).json(&payload).send().await?;

        if !response.status().is_success() {
            return Err(AlertError::NotificationFailed(format!(
                "Telegram API returned status: {}",
                response.status()
            )));
        }

        log::info!("Alert {} sent via Telegram to chat {}", alert.id, chat_id);
        Ok(())
    }

    fn name(&self) -> &'static str {
        "telegram"
    }

    fn is_configured(&self) -> bool {
        self.bot_token.is_some() && self.chat_id.is_some()
    }
}
