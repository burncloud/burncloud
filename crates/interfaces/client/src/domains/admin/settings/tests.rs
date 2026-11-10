use super::{
    model::{AdminSettingsModel, BASE_REV_SHARE, L4_REV_SHARE, TARGET_HEADROOM},
    state::AdminSettingsState,
};
use crate::app::router::routes::Route;

#[test]
fn default_values_match_settings_reference_page() {
    let model = AdminSettingsModel::default();

    assert_eq!(model.base_rev_share, BASE_REV_SHARE);
    assert_eq!(model.l4_rev_share, L4_REV_SHARE);
    assert_eq!(model.target_headroom, TARGET_HEADROOM);
}

#[test]
fn editing_a_policy_updates_values() {
    let mut state = AdminSettingsState::default();

    state.set_base_rev_share("80.0".to_string());
    state.set_l4_rev_share("90.0".to_string());
    state.set_target_headroom("30.0".to_string());

    assert_eq!(state.base_rev_share, "80.0");
    assert_eq!(state.l4_rev_share, "90.0");
    assert_eq!(state.target_headroom, "30.0");
    assert!(!state.saved);
}

#[test]
fn saved_feedback_can_be_marked_and_cleared() {
    let mut state = AdminSettingsState::default();

    state.mark_saved();
    assert!(state.saved);
    state.clear_saved();
    assert!(!state.saved);
}

#[test]
fn admin_settings_route_parses_to_dedicated_component() {
    assert!(matches!(
        "/admin/settings".parse::<Route>(),
        Ok(Route::AdminSettings {})
    ));
}
