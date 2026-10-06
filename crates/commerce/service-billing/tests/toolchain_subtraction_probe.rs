#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! Reports whether this toolchain's signed subtraction clamps, and what the answer means for the repository
//! (#673).
//!
//! ## The environment fact
//!
//! On this machine, `checked_sub` on **signed** integers does not detect underflow, and `saturating_sub` and
//! `overflowing_sub` are implemented on top of it, so all three return ordinary-looking values instead of
//! signalling it. Measured in a dependency-free program outside this repository:
//!
//! ```text
//! 10i64.checked_sub(99)      == Some(-89)          // documented: None
//! 10i64.saturating_sub(99)   == -89                // documented: 0
//! 10i64.overflowing_sub(99)  == (-89, false)       // documented: (i64::MAX, true)
//! ```
//!
//! The same across three toolchains (1.89.0, 1.94.1, 1.96.0-nightly), in debug and release, with no unusual
//! build configuration. Unsigned types are unaffected, as are `checked_add`, `checked_mul`, `checked_div`,
//! `checked_neg` and `checked_abs`.
//!
//! ## Why this is a report rather than an assertion
//!
//! The obvious way to write this test is to assert the broken behaviour, so it is caught in CI. That is
//! backwards: it would pass while the environment is wrong and **fail once the environment is fixed**, which
//! makes repairing the toolchain look like a regression.
//!
//! So the check asserts only what is true on **every** toolchain -- that the hand-written clamp works -- and
//! prints the rest. A run on a sound toolchain prints "correct" and changes nothing. The purpose is to make
//! the environment visible to whoever runs the suite, not to gate on it.
//!
//! ## The audit
//!
//! Nineteen `.saturating_sub(` call sites, and no `checked_sub` or `overflowing_sub` anywhere in `crates/`.
//!
//! | Site | Type | Reachable? |
//! | --- | --- | --- |
//! | `traffic/router/src/passthrough.rs:226` | `u32` | **no** -- unsigned clamps correctly |
//! | `traffic/router/src/rate_budget.rs:184,345,348` | `u64` / `u32` | **no** -- unsigned |
//! | `interfaces/cli/src/cli/monitor.rs:377` | `u32` | **no** -- unsigned |
//! | `commerce/database-billing/src/billing_subscription.rs:270` | `i64` | **only from pre-existing data** |
//! | twelve in `client/src/**` | display counts | yes, but cosmetic |
//!
//! ### Why the billing site is not live
//!
//! `let quota_remaining = sub.quota_limit.saturating_sub(sub.quota_used);` uses two `i64`s, so the broken
//! clamp does apply. What makes it unreachable in normal operation is that `quota_used` has exactly **two**
//! writers in the whole repository, both in `increment_quota`, and both behind the same guard:
//!
//! ```ignore
//! if sub.quota_used + amount > sub.quota_limit {
//!     return Err(DatabaseError::InvalidData { message: "Quota exceeded".to_string() });
//! }
//! ```
//!
//! The guard permits `quota_used == quota_limit` and refuses anything above it, so `quota_remaining` reaches
//! zero but not below. A negative would need the row to be written by something else -- a migration, a manual
//! correction, or a future writer that skips the guard. That makes it a **latent** hazard, and calling it a
//! live bug without a reproduction would be overstating it.
//!
//! ### The client sites
//!
//! `analytics_full.rs`, `catalog.rs`, `logs_full.rs` and `providers.rs` compute display counts such as
//! `unavailable_models`. A negative value there renders as a negative number on a page: worth knowing,
//! cosmetic in effect, and out of scope for a billing audit.

/// Print what this toolchain does, and assert only the property that holds everywhere.
///
/// The assertion is about the **hand-written clamp**, which is correct on any toolchain. The broken forms are
/// printed rather than asserted, so a fixed environment does not turn this red.
#[test]
fn report_whether_signed_subtraction_clamps_on_this_toolchain() {
    let checked = 10i64.checked_sub(99);
    let saturating = 10i64.saturating_sub(99);
    let (overflowing_value, overflowing_flag) = 10i64.overflowing_sub(99);

    // The forms that are correct even here, so the report distinguishes "signed subtraction is broken" from
    // "the machine is odd in general".
    let add_overflow = i64::MAX.checked_add(1);
    let unsigned_sub = 10u64.checked_sub(99);
    let no_underflow = 99i64.checked_sub(10);

    let signed_subtraction_is_sound =
        checked.is_none() && saturating == 0 && overflowing_flag && overflowing_value == i64::MAX;

    println!("signed subtraction on this toolchain:");
    println!("  checked_sub(10, 99)            = {checked:?}          (sound toolchain: None)");
    println!(
        "  saturating_sub(10, 99)         = {saturating}                   (sound toolchain: 0)"
    );
    println!(
        "  overflowing_sub(10, 99)        = ({overflowing_value}, {overflowing_flag})  (sound: (i64::MAX, true))"
    );
    println!(
        "  -> {}",
        if signed_subtraction_is_sound {
            "SOUND"
        } else {
            "BROKEN (see #673)"
        }
    );

    println!("for contrast, the operations that are correct here:");
    println!("  i64::MAX.checked_add(1)        = {add_overflow:?}");
    println!("  10u64.checked_sub(99)          = {unsigned_sub:?}");
    println!("  99i64.checked_sub(10)          = {no_underflow:?}");

    if !signed_subtraction_is_sound {
        println!();
        println!("This means every signed `.saturating_sub(`, `.checked_sub(` and `.overflowing_sub(` in the");
        println!("workspace returns a value instead of signalling underflow. #673 lists the call sites that");
        println!("were audited and why most of them cannot be reached.");
    }

    // The one assertion, true on every toolchain: the explicit clamp is safe, and it is what the audit
    // recommends where a clamp is genuinely needed.
    let a: i64 = 10;
    let b: i64 = 99;
    let via_manual = if a > b { a - b } else { 0 };
    assert_eq!(
        via_manual, 0,
        "the hand-written clamp must always clamp; it is the form the workspace can rely on"
    );
    assert!(
        via_manual <= saturating.max(0),
        "and it must never be larger than a correctly clamped value"
    );
}
