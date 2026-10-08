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
//! **A refund goes back to the bucket that actually supplied the tokens** (#675). `try_consume` takes the
//! estimate from the first bucket that can afford it (`rate_budget.rs`), reporting `OwnBucket` or
//! `Borrowed { from, tpm }`; `BudgetGuard::with_source` records that, and both `commit` and `Drop` return the
//! released tokens to the lender first, then to the requester's own bucket. An own-bucket admission behaves
//! exactly as before.
//!
//! The refund is still capped at each bucket's own reservation, so no capacity can be invented -- only the
//! *destination* changed, which is what the reservation exists to control.
//!
//! Every budget here is freshly configured and the tests do not sleep, so the once-per-minute refill plays no
//! part in any result.

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
///
/// The guard is built `with_source`, exactly as `lib.rs` does it, so the tests exercise the production
/// pairing rather than a hand-assembled "own bucket" assumption.
fn reserve(
    b: &InMemoryBudget,
    color: TrafficColor,
    est_tpm: u64,
) -> (ConsumeOutcome, Option<BudgetGuard<'_>>) {
    let outcome = b.try_consume(1, color, est_tpm);
    let guard = outcome
        .sourced()
        .map(|source| BudgetGuard::with_source(b, 1, color, est_tpm, source));
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

/// Drain yellow so the next yellow request must borrow from green.
fn drain_yellow(b: &InMemoryBudget) {
    let (outcome, guard) = reserve(b, TrafficColor::Yellow, 40_000);
    assert_eq!(outcome, ConsumeOutcome::OwnBucket);
    guard.expect("admitted").commit(40_000);
    assert_eq!(snapshot(b).tpm_remaining_yellow, 0);
}

#[test]
fn a_borrowed_reservation_is_refunded_to_the_lender() {
    // Yellow's own bucket is drained first, so the next yellow request borrows from green. When that request
    // commits an overestimate, the unused part goes back to **green** -- the bucket that supplied the tokens --
    // and yellow, which supplied nothing, is not credited with a refund it did not earn.
    let b = budget_with(ChannelReservation::default());
    drain_yellow(&b);

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

    // It used only half of the estimate, so 5_000 goes back to green.
    guard.expect("admitted").commit(5_000);

    let after = snapshot(&b);
    println!(
        "yellow after the refund: {}, green: {} (borrowed 10_000 from green, used 5_000)",
        after.tpm_remaining_yellow, after.tpm_remaining_green
    );

    assert_eq!(
        after.tpm_remaining_green,
        green_before - 5_000,
        "green lent 10_000 and got 5_000 back, so green is short exactly the 5_000 that was used"
    );
    assert_eq!(
        after.tpm_remaining_yellow, 0,
        "yellow supplied nothing, so it must not be credited with a refund it did not earn"
    );
}

#[test]
fn a_borrowed_cancellation_refunds_to_the_lender_as_well() {
    // The same on the cancel path, which is the more common one for a failed request, and larger because the
    // full estimate comes back rather than the difference.
    let b = budget_with(ChannelReservation::default());
    drain_yellow(&b);

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
        after.tpm_remaining_green, state.tpm_remaining_green,
        "cancelling returns the estimate to green, which lent it, so green is whole again"
    );
    assert_eq!(
        after.tpm_remaining_yellow, state.tpm_remaining_yellow,
        "and yellow, which was empty, is empty again"
    );
}

#[test]
fn a_large_borrow_is_fully_returned_without_losing_capacity() {
    // With a small yellow share the old code refunded at most yellow's own reservation and silently dropped
    // the rest, so the two buckets together held less than before the request until the next refill. The
    // lender-first refund cannot drop anything: it returns exactly what it took.
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
    assert_eq!(snapshot(&b).tpm_remaining_yellow, 0);

    let drained = snapshot(&b);

    let (outcome, guard) = reserve(&b, TrafficColor::Yellow, 30_000);
    assert!(
        matches!(outcome, ConsumeOutcome::Borrowed { .. }),
        "yellow is empty, so this must borrow; got {outcome:?}"
    );
    drop(guard);

    let after = snapshot(&b);
    println!(
        "after borrowing 30_000 and cancelling: yellow {}, green {}",
        after.tpm_remaining_yellow, after.tpm_remaining_green
    );

    assert_eq!(
        after.tpm_remaining_green, drained.tpm_remaining_green,
        "green lent 30_000 and got all of it back"
    );
    assert_eq!(
        after.tpm_remaining_yellow, 0,
        "yellow supplied nothing, so it stays empty rather than being refilled to its cap"
    );
    assert_eq!(
        after.tpm_remaining_green + after.tpm_remaining_yellow,
        drained.tpm_remaining_green + drained.tpm_remaining_yellow,
        "so the two buckets together hold exactly what they held before the cancelled borrow: the capped \
         excess is not dropped anywhere (before the fix this was 30_000 lower until the next refill)"
    );
}

/// A refund may never push a bucket past its own reservation, even when far more is released than was ever
/// borrowed — otherwise a misdirected refund would invent capacity.
#[test]
fn a_borrow_refund_cannot_exceed_the_reserved_cap() {
    let b = budget_with(ChannelReservation::default());
    drain_yellow(&b);

    let (outcome, guard) = reserve(&b, TrafficColor::Yellow, 10_000);
    assert!(matches!(outcome, ConsumeOutcome::Borrowed { .. }));
    // Ask for the whole estimate back even though more than it was borrowed is in play across the channel.
    guard.expect("admitted").commit(0);

    let after = snapshot(&b);
    assert!(
        after.tpm_remaining_green <= 40_000,
        "green must not exceed its 40_000 reservation, got {}",
        after.tpm_remaining_green
    );
    assert!(
        after.tpm_remaining_yellow <= 40_000,
        "yellow must not exceed its 40_000 reservation, got {}",
        after.tpm_remaining_yellow
    );
}

// -------------------------------------------------------------------------------------------
// the decision
// -------------------------------------------------------------------------------------------

/// The property #673-style issues share: a documented mechanism must not silently do something else.
///
/// The contract #675 named: a borrowed reservation is refunded along the borrow chain, so each bucket gets
/// back exactly what it lent and the split between colours (the thing the reservation exists to control) is
/// restored rather than shifted.
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
