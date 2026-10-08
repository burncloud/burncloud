//! Static defaults for the supplier/settings page.

use serde::Deserialize;

pub const NODE_TOKEN: &str = "bc_node_auth_98a72b109e44f128";
pub const ALERT_EMAIL: &str = "ops@datacenter-infra.io";
pub const WEBHOOK_URL: &str = "https://api.datacenter-infra.io/webhooks/burncloud";
pub const INSTALL_COMMAND: &str =
    "curl -sSL https://burncloud.io/install.sh | bash -s -- --token=bc_node_auth_98a72b109e44f128";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SupplierSettingsModel {
    pub node_token: &'static str,
    pub install_command: &'static str,
    pub alert_email: &'static str,
    pub webhook_url: &'static str,
}

impl Default for SupplierSettingsModel {
    fn default() -> Self {
        Self {
            node_token: NODE_TOKEN,
            install_command: INSTALL_COMMAND,
            alert_email: ALERT_EMAIL,
            webhook_url: WEBHOOK_URL,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SavedSupplierSettings {
    pub alert_email: Option<String>,
    pub webhook_url: Option<String>,
}
