#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! Permit release and TPM refund for the rate budget (#633, plan section 5 item 12).
//!
//! The plan asks for "concurrency limit, permit release, tpm refund" and the router's own `shaper_tests.rs`
//! covers the guard from the outside -- `t8_drop_refunds_full_est_on_timeout_cancel` and
//! `t7_commit_refunds_overestimate` drive a request and read the effect. What is not covered is the guard's
//! contract in isolation, and one case in particular that the outside tests cannot see:
//!
//! **A refund goes to the caller's own colour bucket, even when the consumption was borrowed from another.**
//! `try_consume` takes the estimate from the first bucket that can afford it (`rate_budget.rs:381-395`), but
//! `BudgetGuard::commit` refunds the difference to `self.color` (`:186`), and `refund` credits only that one
//! bucket, capped at its own reservation (`:406-411`).
//!
//! So a yellow request that borrowed from green and then used less than estimated returns the difference to
//! **yellow**, which may be back at full reservation, while **green stays short**. No capacity is created or
//! destroyed -- the two buckets are views over one cap -- but the split between them shifts, and the split is
//! what the reservation exists to control.
//!
//! This file measures that rather than fixing it: whether the refund should follow the borrow chain is a
//! decision about what the reservations mean, and the failing contract test at the end names it.
//!
//! Every budget here is freshly configured and the tests do not sleep, so the once-per-minute refill
//! (`:362`) plays no part in any result.

use burncloud_router::rate_budget::{
    BudgetBackend, BudgetGuard, BudgetSnapshot, ChannelReservation, ConsumeOutcome, InMemoryBudget,
};
use burncloud_traffic_contracts::TrafficColor;

/// A budget with one channel and 100_000 TPM, split by the given reservation.
///
/// The default reservation is 0.4 / 0.4 / 0.2, so green gets 40_000, yellow 40_000 and red 20_000.
fn budget_with(reservation: ChannelReservation) -> InMemoryBudget {
    let b = InMemoryBudget::new();
    b.configure(1, 100, 100_000, reservation);
    b
}

fn snapshot(b: &InMemoryBudget) -> BudgetSnapshot {
    b.snapshot(1).expect("channel 1 is configured")
}

/// Consume and immediately build the guard, which is the pair the failover loop performs.
fn reserve(
    b: &InMemoryBudget,
    color: TrafficColor,
    est_tpm: u64,
) -> (ConsumeOutcome, Option<BudgetGuard<'_>>) {
    let outcome = b.try_consume(1, color, est_tpm);
    let guard = if outcome.admitted() {
        Some(BudgetGuard::new(b, 1, color, est_tpm))
    } else {
        None
    };
    (outcome, guard)
}

// -------------------------------------------------------------------------------------------
// the contract the router depends on
// -------------------------------------------------------------------------------------------

#[test]
fn dropping_an_uncommitted_guard_refunds_the_whole_estimate() {
    // The cancellation path: the request was admitted, the upstream never finished, and nothing called
    // `commit`. Without the `Drop` refund those tokens would be held forever by a request that is over.
    let b = budget_with(ChannelReservation::default());

    let before = snapshot(&b).tpm_remaining_green;
    let (outcome, guard) = reserve(&b, TrafficColor::Green, 5_000);
    assert_eq!(outcome, ConsumeOutcome::OwnBucket);
    assert_eq!(
        snapshot(&b).tpm_remaining_green,
        before - 5_000,
        "the reservation is held while the guard is alive"
    );

    drop(guard);

    assert_eq!(
        snapshot(&b).tpm_remaining_green,
        before,
        "dropping without committing must return the whole estimate"
    );
}

#[test]
fn committing_an_overestimate_refunds_only_the_difference() {
    // The success path: the request used less than reserved, so the excess goes back -- and the amount that
    // was actually used stays consumed.
    let b = budget_with(ChannelReservation::default());

    let before = snapshot(&b).tpm_remaining_green;
    let (_, guard) = reserve(&b, TrafficColor::Green, 5_000);
    guard.expect("admitted").commit(3_000);

    assert_eq!(
        snapshot(&b).tpm_remaining_green,
        before - 3_000,
        "2_000 of the 5_000 estimate is refunded and the 3_000 actually used is kept"
    );
}

#[test]
fn committing_exactly_the_estimate_refunds_nothing() {
    // The boundary between the two paths. `to_refund` is zero, so `refund` is not called at all.
    let b = budget_with(ChannelReservation::default());

    let before = snapshot(&b).tpm_remaining_green;
    let (_, guard) = reserve(&b, TrafficColor::Green, 5_000);
    guard.expect("admitted").commit(5_000);

    assert_eq!(snapshot(&b).tpm_remaining_green, before - 5_000);
}

#[test]
fn committing_more_than_the_estimate_does_not_consume_the_extra() {
    // An underestimate. The guard holds only the estimate, so the extra cannot be taken from the bucket after
    // the fact -- the bucket would otherwise be charged a number it never reserved, and no RPM was counted
    // for it either. Recorded because it is the case the code silently ignores rather than reports.
    let b = budget_with(ChannelReservation::default());

    let before = snapshot(&b).tpm_remaining_green;
    let (_, guard) = reserve(&b, TrafficColor::Green, 5_000);
    guard.expect("admitted").commit(9_000);

    assert_eq!(
        snapshot(&b).tpm_remaining_green,
        before - 5_000,
        "the bucket keeps the 5_000 that was reserved; the 4_000 overrun is not charged to it"
    );
}

#[test]
fn a_guard_that_was_committed_does_not_refund_again_on_drop() {
    // `commit` takes `self` and sets `committed`, so `Drop` is a no-op. If it were not, every successful
    // request would return its full estimate *and* keep the actual -- a bucket that only ever grows.
    let b = budget_with(ChannelReservation::default());

    let before = snapshot(&b).tpm_remaining_green;
    let (_, guard) = reserve(&b, TrafficColor::Green, 5_000);
    guard.expect("admitted").commit(3_000);

    let after = snapshot(&b).tpm_remaining_green;
    assert_eq!(after, before - 3_000);

    // Nothing is left to drop, but the assertion is that a second refund is impossible: the only guard is
    // gone, and the snapshot is stable.
    assert_eq!(snapshot(&b).tpm_remaining_green, after);
}

#[test]
fn the_rpm_slot_is_taken_once_and_is_not_returned_by_the_refund() {
    // `refund` credits TPM only (`:406-411`), so the RPM slot consumed by `try_take_from` is never given back
    // -- not on cancel, not on an overestimate. That is the intended reading for an RPM window (the request
    // did occupy a slot) but it is worth pinning, because a request that is cancelled during a retry storm
    // still spends RPM.
    let b = budget_with(ChannelReservation::default());

    let before = snapshot(&b);
    let (_, guard) = reserve(&b, TrafficColor::Green, 1_000);
    assert_eq!(
        snapshot(&b).rpm_remaining_green,
        before.rpm_remaining_green - 1
    );

    drop(guard);

    assert_eq!(
        snapshot(&b).rpm_remaining_green,
        before.rpm_remaining_green - 1,
        "the RPM slot stays consumed after the TPM refund"
    );
    assert_eq!(
        snapshot(&b).tpm_remaining_green,
        before.tpm_remaining_green,
        "while the TPM does come back"
    );
}

// -------------------------------------------------------------------------------------------
// the case the outside tests cannot see: a borrowed consumption
// -------------------------------------------------------------------------------------------

#[test]
fn a_borrowed_reservation_is_refunded_to_the_borrower_not_the_lender() {
    // The finding. Yellow's own bucket is drained first, so the next yellow request borrows from green. When
    // that request commits an overestimate, the difference goes back to **yellow** -- the borrower -- while
    // **green**, which actually supplied the tokens, stays short.
    let b = budget_with(ChannelReservation::default());

    // Drain yellow so the next yellow request must borrow. Yellow holds 40_000.
    let (outcome, guard) = reserve(&b, TrafficColor::Yellow, 40_000);
    assert_eq!(outcome, ConsumeOutcome::OwnBucket);
    guard.expect("admitted").commit(40_000);
    assert_eq!(snapshot(&b).tpm_remaining_yellow, 0);

    let green_before = snapshot(&b).tpm_remaining_green;

    // This one borrows from green.
    let (outcome, guard) = reserve(&b, TrafficColor::Yellow, 10_000);
    assert!(
        matches!(outcome, ConsumeOutcome::Borrowed { .. }),
        "yellow is empty, so this must borrow; got {outcome:?}"
    );
    assert_eq!(
        snapshot(&b).tpm_remaining_green,
        green_before - 10_000,
        "the tokens came out of green"
    );

    // It used only half of the estimate.
    guard.expect("admitted").commit(5_000);

    let after = snapshot(&b);
    println!(
        "yellow after the refund: {}, green: {} (borrowed 10_000 from green, used 5_000)",
        after.tpm_remaining_yellow, after.tpm_remaining_green
    );

    assert_eq!(
        after.tpm_remaining_green,
        green_before - 10_000,
        "green supplied the tokens but received none of the 5_000 refund, so it stays 10_000 short"
    );
    assert_eq!(
        after.tpm_remaining_yellow, 5_000,
        "the refund landed in yellow, which supplied nothing -- yellow is now 5_000 better off than before \
         it borrowed, and green is 5_000 worse off"
    );
}

#[test]
fn a_borrowed_cancellation_refunds_to_the_borrower_as_well() {
    // The same asymmetry on the cancel path, which is the more common one for a failed request, and larger
    // because the full estimate comes back rather than the difference.
    let b = budget_with(ChannelReservation::default());

    let (_, guard) = reserve(&b, TrafficColor::Yellow, 40_000);
    guard.expect("admitted").commit(40_000);
    assert_eq!(snapshot(&b).tpm_remaining_yellow, 0);

    let state = snapshot(&b);
    let (outcome, guard) = reserve(&b, TrafficColor::Yellow, 10_000);
    assert!(matches!(outcome, ConsumeOutcome::Borrowed { .. }));
    drop(guard); // cancelled

    let after = snapshot(&b);
    println!(
        "after a cancelled borrow: yellow {}, green {}",
        after.tpm_remaining_yellow, after.tpm_remaining_green
    );

    assert_eq!(
        after.tpm_remaining_green,
        state.tpm_remaining_green - 10_000,
        "cancelling returns the estimate to yellow, not to green, so green loses the 10_000 for good"
    );
    assert_eq!(
        after.tpm_remaining_yellow, 10_000,
        "and yellow is credited 10_000 it never had"
    );
}

#[test]
fn the_refund_is_capped_at_the_borrowers_own_reservation() {
    // The cap in `refund` (`:409`) limits how much of a misdirected refund the borrower can absorb. With a
    // small yellow share the excess is dropped entirely, so the tokens are neither with the borrower nor with
    // the lender -- they are gone from both views until the next refill.
    let reservation = ChannelReservation {
        green: 0.8,
        yellow: 0.1,
        red: 0.1,
    };
    let b = budget_with(reservation);
    let snap = snapshot(&b);
    println!(
        "green {} yellow {} red {} (of {})",
        snap.tpm_remaining_green, snap.tpm_remaining_yellow, snap.tpm_remaining_red, snap.tpm_cap
    );

    // Yellow holds 10_000. Spend it, then borrow 30_000 from green and cancel.
    let (_, guard) = reserve(&b, TrafficColor::Yellow, 10_000);
    guard.expect("admitted").commit(10_000);

    let (outcome, guard) = reserve(&b, TrafficColor::Yellow, 30_000);
    assert!(
        matches!(outcome, ConsumeOutcome::Borrowed { .. }),
        "yellow is empty, so this must borrow; got {outcome:?}"
    );
    drop(guard);

    let after = snapshot(&b);
    println!(
        "after borrowing 30_000 and cancelling: yellow {} (cap 10_000), green {}",
        after.tpm_remaining_yellow, after.tpm_remaining_green
    );

    assert_eq!(
        after.tpm_remaining_yellow, 10_000,
        "yellow is refilled to its own reservation, and the other 20_000 of the refund is discarded by the cap"
    );
    assert_eq!(
        after.tpm_remaining_green,
        snap.tpm_remaining_green - 30_000,
        "green is still missing all 30_000 it lent"
    );
    assert!(
        after.tpm_remaining_green + after.tpm_remaining_yellow < snap.tpm_remaining_green + snap.tpm_remaining_yellow,
        "so the two buckets together hold less than they did before the request: the capped excess is not \
         credited anywhere"
    );
}

// -------------------------------------------------------------------------------------------
// the decision
// -------------------------------------------------------------------------------------------

/// The property #673-style issues share: a documented mechanism must not silently do something else.
///
/// **Ignored, not deleted**, and failing today. `BudgetGuard::commit` documents "refund the difference"
/// without saying *to which bucket*, and the answer is "the caller's" rather than "the one that supplied
/// them". Two resolutions are defensible and the choice is about what a reservation means:
///
/// * refund along the borrow chain, so each bucket gets back what it lent;
/// * or keep the current credit but document it, if the buckets are only ever read in total.
///
/// What the test rejects is the current state: an asymmetry that silently moves capacity between the colours
/// the reservation exists to separate, with no warning and no test.
#[ignore = "the contract: a borrowed reservation must be refunded along the borrow chain"]
#[test]
fn a_refund_follows_where_the_tokens_came_from() {
    let b = budget_with(ChannelReservation::default());

    let (_, guard) = reserve(&b, TrafficColor::Yellow, 40_000);
    guard.expect("admitted").commit(40_000);

    let before = snapshot(&b);
    let (outcome, guard) = reserve(&b, TrafficColor::Yellow, 10_000);
    assert!(matches!(outcome, ConsumeOutcome::Borrowed { .. }));
    drop(guard);

    let after = snapshot(&b);
    assert_eq!(
        after.tpm_remaining_green, before.tpm_remaining_green,
        "green lent 10_000 and the request was cancelled, so green must be whole again: {} vs {}",
        after.tpm_remaining_green, before.tpm_remaining_green
    );
    assert_eq!(
        after.tpm_remaining_yellow, before.tpm_remaining_yellow,
        "and yellow, which was empty, must be empty again rather than credited with a refund it did not earn"
    );
}
