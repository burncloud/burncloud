use super::{
    model::{AdminSettlementsModel, SettlementBatchStatus},
    state::AdminSettlementsState,
};
use crate::app::router::routes::Route;

#[test]
fn reference_batches_match_the_admin_settlements_page() {
    let model = AdminSettlementsModel::reference();

    assert_eq!(
        model.batches.map(|batch| (
            batch.id,
            batch.billing_period,
            batch.total_suppliers,
            batch.total_gpus,
            batch.inference_tokens,
            batch.total_payout,
            batch.status
        )),
        [
            (
                "BATCH-2026-08B",
                "Aug 15 - Aug 31, 2026",
                24,
                420,
                "1,840.5M",
                "$124,500.00",
                SettlementBatchStatus::PendingReview
            ),
            (
                "BATCH-2026-08A",
                "Aug 01 - Aug 15, 2026",
                24,
                412,
                "1,790.0M",
                "$121,200.00",
                SettlementBatchStatus::SettledAndPaid
            ),
        ]
    );
}

#[test]
fn approval_updates_only_the_pending_batch() {
    let mut state = AdminSettlementsState::default();
    let previously_settled = state.batches[1];

    state.approve_batch("BATCH-2026-08B");

    assert_eq!(
        state.batches[0].status,
        SettlementBatchStatus::SettledAndPaid
    );
    assert_eq!(state.batches[1], previously_settled);

    state.approve_batch("BATCH-2026-08A");
    assert_eq!(state.batches[1], previously_settled);
}

#[test]
fn admin_settlements_route_parses_to_the_dedicated_page() {
    assert!(matches!(
        "/admin/settlements".parse::<Route>(),
        Ok(Route::AdminSettlements {})
    ));
}
