use super::{
    actions::{format_amount, remaining_days, validate_top_up_amount},
    model::BillingModel,
    state::{BalanceState, BillingState, TopUpError, PRESET_TOP_UP_AMOUNTS},
};
use crate::app::router::routes::Route;

#[test]
fn reference_model_matches_the_billing_page() {
    let model = BillingModel::reference();
    assert_eq!(model.prepaid_balance, Some(128.50));
    assert_eq!(model.today_spend, 14.28);
    assert_eq!(model.auto_top_up_amount, 100.0);
    assert_eq!(model.auto_top_up_threshold, 20.0);
    assert_eq!(model.invoices.len(), 3);
    assert_eq!(model.invoices[0].id, "INV-2026-0882");
    assert_eq!(model.invoices[2].payment_method, "Corporate Wire (ACH)");
    assert!(model.invoices.iter().all(|invoice| invoice.pdf_available));
    assert_eq!(
        model.platform_markup_note,
        "0% Platform markup on frontier models"
    );
}

#[test]
fn default_state_matches_reference_modal_and_balance() {
    let state = BillingState::default();
    assert_eq!(state.balance, Some(128.50));
    assert_eq!(state.today_spend, 14.28);
    assert_eq!(state.selected_amount, 100.0);
    assert!(state.custom_amount.is_empty());
    assert!(!state.modal_open);
    assert!(state.auto_recharge_enabled);
    assert_eq!(state.balance_state(), BalanceState::Healthy);
}

#[test]
fn presets_and_custom_amounts_follow_reference_behavior() {
    let mut state = BillingState::default();
    assert_eq!(PRESET_TOP_UP_AMOUNTS, [50.0, 100.0, 250.0, 500.0, 1000.0]);
    state.set_custom_amount("1500".to_string());
    assert_eq!(state.amount_for_display(), 1500.0);
    state.select_preset(250.0);
    assert_eq!(state.amount_for_display(), 250.0);
    assert!(state.custom_amount.is_empty());
}

#[test]
fn top_up_validation_and_submission_are_safe() {
    assert_eq!(validate_top_up_amount(10.0), Ok(()));
    assert_eq!(validate_top_up_amount(9.99), Err(TopUpError::BelowMinimum));
    assert_eq!(
        validate_top_up_amount(f64::NAN),
        Err(TopUpError::InvalidAmount)
    );

    let mut state = BillingState::default();
    state.open_modal();
    state.set_custom_amount("25.50".to_string());
    assert_eq!(state.submit_top_up(), Ok(25.50));
    assert_eq!(state.balance, Some(154.0));
    assert!(!state.modal_open);
    assert!(state.custom_amount.is_empty());
}

#[test]
fn invalid_top_up_does_not_change_balance() {
    let mut state = BillingState::default();
    state.open_modal();
    state.set_custom_amount("5".to_string());
    assert_eq!(state.submit_top_up(), Err(TopUpError::BelowMinimum));
    assert_eq!(state.balance, Some(128.50));
    assert!(state.modal_open);
}

#[test]
fn toggle_and_success_feedback_are_local_state_only() {
    let mut state = BillingState::default();
    state.toggle_auto_recharge();
    assert!(!state.auto_recharge_enabled);
    state.set_success_message("done".to_string());
    let first_version = state.success_message_version;
    assert_eq!(state.success_message.as_deref(), Some("done"));
    state.set_success_message("done".to_string());
    let latest_version = state.success_message_version;
    state.clear_success_message_if(first_version);
    assert_eq!(state.success_message.as_deref(), Some("done"));
    state.clear_success_message_if(latest_version);
    assert!(state.success_message.is_none());
    state.clear_success_message();
    assert!(state.success_message.is_none());
}

#[test]
fn balance_states_and_remaining_days_are_explicit() {
    assert_eq!(BalanceState::from_balance(None), BalanceState::Unavailable);
    assert_eq!(
        BalanceState::from_balance(Some(f64::NAN)),
        BalanceState::Unavailable
    );
    assert!(BalanceState::Unavailable.is_alert());
    assert_eq!(
        BalanceState::from_balance(Some(0.0)),
        BalanceState::Critical
    );
    assert_eq!(BalanceState::from_balance(Some(19.99)), BalanceState::Low);
    assert_eq!(
        BalanceState::from_balance(Some(20.0)),
        BalanceState::Healthy
    );
    assert_eq!(remaining_days(Some(128.50), 14.28), Some(8));
    assert_eq!(remaining_days(None, 14.28), None);
    assert_eq!(remaining_days(Some(f64::NAN), 14.28), None);
    assert_eq!(format_amount(25.5), "$25.50");
}

#[test]
fn billing_route_parses_to_the_dedicated_component() {
    assert!(matches!(
        "/buyer/billing".parse::<Route>(),
        Ok(Route::Billing {})
    ));
}
