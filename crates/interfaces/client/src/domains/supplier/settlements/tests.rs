use super::{
    actions::format_currency, model::SupplierSettlementsModel, state::SupplierSettlementsState,
};
use crate::app::router::routes::Route;

#[test]
fn reference_settlements_match_the_supplier_page() {
    let model = SupplierSettlementsModel::reference();

    assert_eq!(model.next_payout, 6_420.00);
    assert_eq!(model.payout_destination, "Silicon Valley Bank (ACH)");
    assert_eq!(model.completed_batches, 12);
    assert_eq!(model.batches.len(), 3);
    assert_eq!(model.batches[0].id, "SET-2026-0815");
    assert_eq!(model.batches[2].payment_method, "USDC (0x9a8f...3d11)");
    assert_eq!(format_currency(model.batches[1].amount), "$7,120.50");
}

#[test]
fn payout_state_is_local_and_resets_after_cancel_or_confirm() {
    let mut state = SupplierSettlementsState::default();
    assert!(!state.payout_modal_open);
    assert!(state.success_message.is_none());

    state.open_payout_modal();
    assert!(state.payout_modal_open);
    state.close_payout_modal();
    assert!(!state.payout_modal_open);

    state.open_payout_modal();
    state.confirm_payout("Payout request queued");
    assert!(!state.payout_modal_open);
    assert_eq!(
        state.success_message.as_deref(),
        Some("Payout request queued")
    );
    state.clear_success_message();
    assert!(state.success_message.is_none());
}

#[test]
fn supplier_settlements_route_parses_to_the_dedicated_page() {
    assert!(matches!(
        "/supplier/settlements".parse::<Route>(),
        Ok(Route::SupplierSettlements {})
    ));
}
