//! Tests for the supplier/settings page boundary.

use super::{
    model::{SupplierSettingsModel, ALERT_EMAIL, INSTALL_COMMAND, NODE_TOKEN, WEBHOOK_URL},
    state::SupplierSettingsState,
};
use crate::app::router::routes::Route;

#[test]
fn reference_settings_match_supplier_page() {
    let model = SupplierSettingsModel::default();

    assert_eq!(model.node_token, NODE_TOKEN);
    assert_eq!(model.install_command, INSTALL_COMMAND);
    assert_eq!(model.alert_email, ALERT_EMAIL);
    assert_eq!(model.webhook_url, WEBHOOK_URL);
    assert!(model.install_command.contains(model.node_token));
}

#[test]
fn settings_state_updates_and_resets_feedback() {
    let mut state = SupplierSettingsState::default();

    state.mark_command_copied();
    assert!(state.command_copied);
    state.clear_command_copied();
    assert!(!state.command_copied);

    state.mark_saved();
    assert!(state.saved);
    state.set_webhook_url("https://example.com/hook");
    assert!(!state.saved);
    assert_eq!(state.webhook_url, "https://example.com/hook");
}

#[test]
fn supplier_settings_route_parses_to_dedicated_component() {
    assert!(matches!(
        "/supplier/settings".parse::<Route>(),
        Ok(Route::SupplierSettings {})
    ));
}
