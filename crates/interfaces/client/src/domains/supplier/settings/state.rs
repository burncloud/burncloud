//! Local state for the supplier/settings page.

use super::model::{SavedSupplierSettings, SupplierSettingsModel, ALERT_EMAIL};

#[derive(Clone, Debug, PartialEq)]
pub struct SupplierSettingsState {
    pub model: SupplierSettingsModel,
    pub alert_email: String,
    pub webhook_url: String,
    pub command_copied: bool,
    pub saved: bool,
}

impl Default for SupplierSettingsState {
    fn default() -> Self {
        let model = SupplierSettingsModel::default();
        Self {
            model,
            alert_email: model.alert_email.to_string(),
            webhook_url: model.webhook_url.to_string(),
            command_copied: false,
            saved: false,
        }
    }
}

impl SupplierSettingsState {
    pub fn effective_alert_email(&self) -> &str {
        if self.alert_email.is_empty() {
            ALERT_EMAIL
        } else {
            &self.alert_email
        }
    }

    pub fn set_alert_email(&mut self, value: impl Into<String>) {
        self.alert_email = value.into();
        self.saved = false;
    }

    pub fn set_webhook_url(&mut self, value: impl Into<String>) {
        self.webhook_url = value.into();
        self.saved = false;
    }

    pub fn apply_saved(&mut self, saved: SavedSupplierSettings) {
        if let Some(email) = saved.alert_email.filter(|value| !value.is_empty()) {
            self.alert_email = email;
        }
        if let Some(webhook) = saved.webhook_url.filter(|value| !value.is_empty()) {
            self.webhook_url = webhook;
        }
    }

    pub fn mark_command_copied(&mut self) {
        self.command_copied = true;
    }

    pub fn clear_command_copied(&mut self) {
        self.command_copied = false;
    }

    pub fn mark_saved(&mut self) {
        self.saved = true;
    }

    pub fn clear_saved(&mut self) {
        self.saved = false;
    }
}
