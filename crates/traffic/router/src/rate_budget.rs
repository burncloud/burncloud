//! L2 Shaper — three-color token-bucket rate budget per (channel, model).
//!
//! Implements the MPLS TE bandwidth model: each channel has a total RPM/TPM
//! capacity split into three reservations (Green / Yellow / Red). A request
//! tagged with [`TrafficColor`] tries its own bucket first; if empty it may
//! **borrow** idle capacity from higher-priority colors (Green > Yellow > Red).
//!
//! # Scope (MVP — 审查 E-D6)
//!
//! - ✅ Three-color buckets with reservations
//! - ✅ Borrow from higher-priority color when own bucket empty
//! - ❌ **Preemption deferred** — high-priority arrival cannot reclaim a token
//!   already lent out. Adding preemption requires per-bucket consistency
//!   semantics (CAS loops) that are easy to get wrong; defer to phase 2.5
//!   or a dedicated PR.
//!
//! # Backend abstraction (审查 E-D4 / 路径 2 forward-compat)
//!
//! [`BudgetBackend`] is the trait. [`InMemoryBudget`] is the only impl shipped
//! in MVP. A future `RedisBudget` (phase 4) plugs in without rewriting the
//! consumers — see `docs/design/channel-scheduler-hqos.md` § 阶段 2.
//!
//! # Glossary alignment
//!
//! See `docs/code/GLOSSARY.md` § 2: this is the **trTCM** (two-rate three-color
//! marker, RFC 2698) shape. Future rename: [`InMemoryBudget`] → `TrTCMShaper`.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use burncloud_traffic_contracts::TrafficColor;
use dashmap::DashMap;

/// Result of a [`BudgetBackend::try_consume`] attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumeOutcome {
    /// Tokens taken from the request's own color bucket.
    OwnBucket,
    /// Tokens taken from a higher-priority color's idle reservation.
    ///
    /// `tpm` is how much of the estimate the lender actually supplied: the
    /// whole estimate, because a borrow either takes all of it or is not
    /// attempted. It is carried explicitly so the matching
    /// [`BudgetGuard`] can return the tokens to *that* bucket — see
    /// `BudgetGuard::with_source`.
    Borrowed { from: TrafficColor, tpm: u64 },
    /// All buckets that this color may draw from are empty.
    /// Caller should reject the request with 503 + `X-Rejected-By: shaper`.
    Rejected,
}

impl ConsumeOutcome {
    /// `true` for `OwnBucket` and `Borrowed`, `false` for `Rejected`.
    pub fn admitted(&self) -> bool {
        !matches!(self, ConsumeOutcome::Rejected)
    }

    /// Where this admission actually took its TPM from.
    ///
    /// `None` for [`ConsumeOutcome::Rejected`] — nothing was consumed, so
    /// there is nothing to build a [`BudgetGuard`] for. Callers should pair
    /// this with [`BudgetGuard::with_source`] instead of assuming the
    /// request's own color supplied the tokens.
    pub fn sourced(&self) -> Option<ReservationSource> {
        match *self {
            ConsumeOutcome::OwnBucket => Some(ReservationSource::Own),
            ConsumeOutcome::Borrowed { from, tpm } => {
                Some(ReservationSource::Borrowed { from, tpm })
            }
            ConsumeOutcome::Rejected => None,
        }
    }

    /// Static label for `router_logs.layer_decision`.
    pub fn as_label(&self) -> &'static str {
        match self {
            ConsumeOutcome::OwnBucket => "shaper_own",
            ConsumeOutcome::Borrowed { .. } => "shaper_borrow",
            ConsumeOutcome::Rejected => "shaper_reject",
        }
    }
}

/// Which bucket actually supplied the TPM a [`BudgetGuard`] is holding.
///
/// Recorded at admission time and used at release time, so that a refund
/// follows the borrow chain instead of always landing in the requester's own
/// color (#675).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationSource {
    /// The requester's own color bucket supplied everything.
    Own,
    /// Another color's idle reservation supplied `tpm` tokens.
    Borrowed { from: TrafficColor, tpm: u64 },
}

/// Per-channel reservation policy. Three values must sum to 1.0 (±epsilon).
#[derive(Debug, Clone, Copy)]
pub struct ChannelReservation {
    pub green: f64,
    pub yellow: f64,
    pub red: f64,
}

impl Default for ChannelReservation {
    /// Mirror the migration defaults: 40% / 40% / 20%.
    fn default() -> Self {
        Self {
            green: 0.4,
            yellow: 0.4,
            red: 0.2,
        }
    }
}

impl ChannelReservation {
    /// Validate sum-to-1.0 (±0.01). Use after loading from DB.
    pub fn is_valid(&self) -> bool {
        let sum = self.green + self.yellow + self.red;
        (0.99..=1.01).contains(&sum) && self.green >= 0.0 && self.yellow >= 0.0 && self.red >= 0.0
    }

    /// Return the share for a specific color.
    pub fn share(&self, color: TrafficColor) -> f64 {
        match color {
            TrafficColor::Green => self.green,
            TrafficColor::Yellow => self.yellow,
            TrafficColor::Red => self.red,
        }
    }
}

/// Per-channel snapshot for observability. Returned by [`BudgetBackend::snapshot`].
#[derive(Debug, Clone, Copy)]
pub struct BudgetSnapshot {
    pub rpm_cap: u32,
    pub rpm_remaining_green: u32,
    pub rpm_remaining_yellow: u32,
    pub rpm_remaining_red: u32,
    pub tpm_cap: u64,
    pub tpm_remaining_green: u64,
    pub tpm_remaining_yellow: u64,
    pub tpm_remaining_red: u64,
}

/// Pluggable backend for the rate-budget Shaper.
///
/// MVP ships [`InMemoryBudget`]. Phase 4 adds `RedisBudget` for multi-instance
/// fleets — same trait, no rewrite needed at the call site.
pub trait BudgetBackend: Send + Sync {
    /// Atomically attempt to consume `tokens` RPM (1 unit = 1 request) +
    /// `est_tpm` TPM (estimated tokens for this request).
    /// Returns the outcome — caller checks [`ConsumeOutcome::admitted`].
    fn try_consume(&self, channel_id: i32, color: TrafficColor, est_tpm: u64) -> ConsumeOutcome;

    /// Refund (return tokens to the bucket). Used after the response when the
    /// estimated TPM was higher than the actual usage — read `router_logs.cost`
    /// or token counters and call `refund(channel, color, est - actual)`.
    ///
    /// The credited bucket is always `color` and the credit is capped at that
    /// bucket's own reservation. That is correct for tokens the bucket itself
    /// supplied; a **borrowed** reservation must be returned to its lender
    /// instead, which is what [`BudgetGuard::with_source`] does (it issues one
    /// capped `refund` per bucket involved). See #675.
    fn refund(&self, channel_id: i32, color: TrafficColor, tpm_to_return: u64);

    /// Read-only snapshot of remaining capacity per color (for `/router/status`).
    fn snapshot(&self, channel_id: i32) -> Option<BudgetSnapshot>;
}

/// RAII guard that ensures the TPM reservation taken by [`BudgetBackend::try_consume`]
/// is returned even if the request never reaches `commit` — i.e. on client
/// cancellation, upstream timeout, or panic during the proxy `.await`.
///
/// **Lifecycle:**
/// - `BudgetGuard::with_source(...)` records the `(channel_id, color, est_tpm)`
///   triple right after a successful `try_consume` (caller already holds the
///   bucket), plus which bucket supplied the tokens.
/// - On the happy path, the caller calls `guard.commit(actual_tpm)`. If the
///   actual TPM was less than the estimate, the over-estimate
///   (`est - actual`) is refunded. The `committed=true` flag stops `Drop`
///   from double-refunding.
/// - On any other path (early `return`, `?` propagation, panic, async cancel),
///   `Drop::drop` runs with `committed=false` and refunds the full `est_tpm`,
///   so the bucket is never permanently held by a request that didn't run.
///
/// **Why `commit(self)` takes self by value:** Once committed, the guard is
/// consumed at the type-system level. There is no way to call `commit` twice
/// or to `commit` and then drop with extra refund — the borrow checker
/// rejects it. This is the audit-flagged FM4 fix (Plan-易漏 client cancel).
///
/// The guard is `Send + Sync` so it may be held across `.await` points in
/// the failover loop without any wrapping.
pub struct BudgetGuard<'a> {
    backend: &'a (dyn BudgetBackend + Send + Sync),
    channel_id: i32,
    color: TrafficColor,
    est_tpm: u64,
    /// Where the estimate was taken from. `Borrowed` refunds to the lender
    /// first, so the reservation split the Shaper exists to enforce is
    /// restored rather than shifted (#675).
    source: ReservationSource,
    committed: bool,
}

impl<'a> BudgetGuard<'a> {
    /// Wrap a freshly-consumed reservation, assuming the requester's own
    /// bucket supplied it.
    ///
    /// Prefer [`BudgetGuard::with_source`] at real call sites: it takes the
    /// [`ConsumeOutcome`]'s own account of where the tokens came from, so a
    /// borrowed admission refunds to the lender. This constructor stays for
    /// callers that consumed from their own bucket and for the guard's
    /// unit-level tests.
    ///
    /// Call only when `try_consume` returned `OwnBucket`. Borrowed
    /// admissions must use [`BudgetGuard::with_source`] so refunds cannot be
    /// misdirected back to the borrower.
    pub fn new_own(
        backend: &'a (dyn BudgetBackend + Send + Sync),
        channel_id: i32,
        color: TrafficColor,
        est_tpm: u64,
    ) -> Self {
        Self::with_source(backend, channel_id, color, est_tpm, ReservationSource::Own)
    }

    /// Wrap a freshly-consumed reservation together with the bucket it came
    /// from — the pairing the failover loop performs after `try_consume`.
    pub fn with_source(
        backend: &'a (dyn BudgetBackend + Send + Sync),
        channel_id: i32,
        color: TrafficColor,
        est_tpm: u64,
        source: ReservationSource,
    ) -> Self {
        Self {
            backend,
            channel_id,
            color,
            est_tpm,
            source,
            committed: false,
        }
    }

    /// Commit the reservation with the actual TPM consumed. If the actual was
    /// smaller than the estimate, refund the difference along the borrow
    /// chain (lender first, then the requester's own bucket). Marks the guard
    /// as committed so `Drop` is a no-op.
    ///
    /// Takes `self` by value — the guard is consumed and cannot be reused.
    pub fn commit(mut self, actual_tpm: u64) {
        let to_refund = self.est_tpm.saturating_sub(actual_tpm);
        if to_refund > 0 {
            self.refund_along_source(to_refund);
        }
        self.committed = true;
        // self drops here — Drop sees `committed = true` and skips full refund.
    }

    /// Return `tpm_to_return` to the buckets that actually supplied it.
    ///
    /// A borrowed reservation goes back to the lender first, capped by what
    /// the lender lent; only an excess the lender cannot absorb (it would
    /// exceed its own reservation because another request refilled it
    /// meanwhile) falls through to the requester's own bucket. Own-bucket
    /// admissions keep the original behaviour exactly.
    fn refund_along_source(&self, tpm_to_return: u64) {
        match self.source {
            ReservationSource::Borrowed { from, tpm } => {
                let back_to_lender = tpm_to_return.min(tpm);
                if back_to_lender > 0 {
                    self.backend.refund(self.channel_id, from, back_to_lender);
                }
                let remainder = tpm_to_return.saturating_sub(back_to_lender);
                if remainder > 0 {
                    self.backend.refund(self.channel_id, self.color, remainder);
                }
            }
            ReservationSource::Own => {
                self.backend
                    .refund(self.channel_id, self.color, tpm_to_return);
            }
        }
    }
}

impl<'a> Drop for BudgetGuard<'a> {
    fn drop(&mut self) {
        if !self.committed && self.est_tpm > 0 {
            // Cancel / panic / early-return path: nothing was committed by the
            // caller, so refund the full estimate — to the lender first when
            // the tokens were borrowed. Otherwise `est_tpm` would be
            // permanently held by a request the upstream never finished, and
            // the split between colours would be silently shifted (#675).
            self.refund_along_source(self.est_tpm);
        }
    }
}

/// Single-instance in-memory bucket. Refills linearly toward the cap once per
/// minute (RPM is a per-minute window — the simplest correct semantic).
///
/// Concurrency: per-channel `Mutex` (DashMap entry granularity). Contention
/// is bounded by candidate-set size (≤ 20 channels per model in practice).
pub struct InMemoryBudget {
    channels: DashMap<i32, Mutex<ChannelBuckets>>,
}

impl Default for InMemoryBudget {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryBudget {
    pub fn new() -> Self {
        Self {
            channels: DashMap::new(),
        }
    }

    /// Configure (or update) the per-channel cap + reservation policy.
    /// Call this when `channel_providers` is loaded / refreshed.
    ///
    /// Invalid reservation (sum != 1.0 ± 0.01) → fallback to
    /// [`ChannelReservation::default`] (0.4/0.4/0.2) with a warning. Prior
    /// behavior was a debug-only `debug_assert!`, which silently passed in
    /// release mode and let bad DB rows produce skewed buckets in prod
    /// (audit FM8 — release-mode silent acceptance).
    pub fn configure(
        &self,
        channel_id: i32,
        rpm_cap: u32,
        tpm_cap: u64,
        reservation: ChannelReservation,
    ) {
        let reservation = if reservation.is_valid() {
            reservation
        } else {
            tracing::warn!(
                channel_id,
                green = reservation.green,
                yellow = reservation.yellow,
                red = reservation.red,
                "rate_budget: invalid reservation (sum != 1.0), falling back to default 0.4/0.4/0.2"
            );
            ChannelReservation::default()
        };
        let buckets = ChannelBuckets::new(rpm_cap, tpm_cap, reservation);
        self.channels.insert(channel_id, Mutex::new(buckets));
    }

    /// Returns true if the channel has been configured.
    pub fn is_configured(&self, channel_id: i32) -> bool {
        self.channels.contains_key(&channel_id)
    }
}

impl BudgetBackend for InMemoryBudget {
    fn try_consume(&self, channel_id: i32, color: TrafficColor, est_tpm: u64) -> ConsumeOutcome {
        // Unconfigured channel = unlimited (fail-open during MVP rollout).
        let entry = match self.channels.get(&channel_id) {
            Some(e) => e,
            None => return ConsumeOutcome::OwnBucket,
        };
        // Mutex is the right tool here: bucket update is multi-step (refill
        // + try-take + maybe-borrow). DashMap's atomicity is per-key.
        let mut buckets = entry.value().lock().unwrap_or_else(|e| {
            // Poisoned mutex: Shaper is best-effort, recover and continue.
            tracing::warn!(channel_id, "rate_budget mutex poisoned, recovering");
            e.into_inner()
        });
        buckets.refill_if_due();
        buckets.try_consume(color, est_tpm)
    }

    fn refund(&self, channel_id: i32, color: TrafficColor, tpm_to_return: u64) {
        if let Some(entry) = self.channels.get(&channel_id) {
            let mut buckets = entry.value().lock().unwrap_or_else(|e| e.into_inner());
            buckets.refund(color, tpm_to_return);
        }
    }

    fn snapshot(&self, channel_id: i32) -> Option<BudgetSnapshot> {
        let entry = self.channels.get(&channel_id)?;
        let buckets = entry.value().lock().unwrap_or_else(|e| e.into_inner());
        Some(BudgetSnapshot {
            rpm_cap: buckets.rpm_cap,
            rpm_remaining_green: buckets.rpm_remaining[color_idx(TrafficColor::Green)],
            rpm_remaining_yellow: buckets.rpm_remaining[color_idx(TrafficColor::Yellow)],
            rpm_remaining_red: buckets.rpm_remaining[color_idx(TrafficColor::Red)],
            tpm_cap: buckets.tpm_cap,
            tpm_remaining_green: buckets.tpm_remaining[color_idx(TrafficColor::Green)],
            tpm_remaining_yellow: buckets.tpm_remaining[color_idx(TrafficColor::Yellow)],
            tpm_remaining_red: buckets.tpm_remaining[color_idx(TrafficColor::Red)],
        })
    }
}

// --- internal --------------------------------------------------------------

const COLOR_COUNT: usize = 3;

#[inline]
fn color_idx(c: TrafficColor) -> usize {
    match c {
        TrafficColor::Green => 0,
        TrafficColor::Yellow => 1,
        TrafficColor::Red => 2,
    }
}

#[inline]
fn idx_color(i: usize) -> TrafficColor {
    match i {
        0 => TrafficColor::Green,
        1 => TrafficColor::Yellow,
        _ => TrafficColor::Red,
    }
}

/// One-minute refill window. Larger window = burstier; smaller = jittery.
/// 60s matches "RPM" semantics (per-minute) directly.
const REFILL_WINDOW: Duration = Duration::from_secs(60);

struct ChannelBuckets {
    rpm_cap: u32,
    tpm_cap: u64,
    rpm_reserved: [u32; COLOR_COUNT],
    tpm_reserved: [u64; COLOR_COUNT],
    rpm_remaining: [u32; COLOR_COUNT],
    tpm_remaining: [u64; COLOR_COUNT],
    last_refill: Instant,
}

impl ChannelBuckets {
    fn new(rpm_cap: u32, tpm_cap: u64, reservation: ChannelReservation) -> Self {
        let rpm_g = ((rpm_cap as f64) * reservation.green) as u32;
        let rpm_y = ((rpm_cap as f64) * reservation.yellow) as u32;
        let rpm_r = rpm_cap.saturating_sub(rpm_g + rpm_y);
        let tpm_g = ((tpm_cap as f64) * reservation.green) as u64;
        let tpm_y = ((tpm_cap as f64) * reservation.yellow) as u64;
        let tpm_r = tpm_cap.saturating_sub(tpm_g + tpm_y);
        Self {
            rpm_cap,
            tpm_cap,
            rpm_reserved: [rpm_g, rpm_y, rpm_r],
            tpm_reserved: [tpm_g, tpm_y, tpm_r],
            rpm_remaining: [rpm_g, rpm_y, rpm_r],
            tpm_remaining: [tpm_g, tpm_y, tpm_r],
            last_refill: Instant::now(),
        }
    }

    /// Top up to reserved levels if a full window has elapsed. Simple
    /// per-minute refill — not GCRA-smooth, but cheap and predictable.
    fn refill_if_due(&mut self) {
        if self.last_refill.elapsed() < REFILL_WINDOW {
            return;
        }
        for i in 0..COLOR_COUNT {
            self.rpm_remaining[i] = self.rpm_reserved[i];
            self.tpm_remaining[i] = self.tpm_reserved[i];
        }
        self.last_refill = Instant::now();
    }

    /// Try own bucket first, then borrow from higher-priority colors.
    /// Borrow direction (audit decision: Green > Yellow > Red, but lower may
    /// borrow upward only — Red may borrow Yellow then Green; Yellow may
    /// borrow Green; Green never borrows down):
    ///
    /// - `Green`  → own only
    /// - `Yellow` → own → Green
    /// - `Red`    → own → Yellow → Green
    fn try_consume(&mut self, color: TrafficColor, est_tpm: u64) -> ConsumeOutcome {
        let own = color_idx(color);
        if self.try_take_from(own, est_tpm) {
            return ConsumeOutcome::OwnBucket;
        }
        // Borrow chain — only upward (lower-priority borrows from higher).
        for &borrowed_idx in borrow_chain(color) {
            if self.try_take_from(borrowed_idx, est_tpm) {
                return ConsumeOutcome::Borrowed {
                    from: idx_color(borrowed_idx),
                    // A borrow takes the whole estimate from one bucket, so
                    // the lender's exposure is exactly `est_tpm`.
                    tpm: est_tpm,
                };
            }
        }
        ConsumeOutcome::Rejected
    }

    fn try_take_from(&mut self, idx: usize, est_tpm: u64) -> bool {
        if self.rpm_remaining[idx] == 0 || self.tpm_remaining[idx] < est_tpm {
            return false;
        }
        self.rpm_remaining[idx] -= 1;
        self.tpm_remaining[idx] -= est_tpm;
        true
    }

    fn refund(&mut self, color: TrafficColor, tpm_to_return: u64) {
        let i = color_idx(color);
        let cap = self.tpm_reserved[i];
        let new_val = self.tpm_remaining[i].saturating_add(tpm_to_return).min(cap);
        self.tpm_remaining[i] = new_val;
    }
}

/// The borrow chain for each color. See `try_consume` doc-comment.
fn borrow_chain(color: TrafficColor) -> &'static [usize] {
    static GREEN: [usize; 0] = [];
    static YELLOW: [usize; 1] = [0]; // Green
    static RED: [usize; 2] = [1, 0]; // Yellow → Green
    match color {
        TrafficColor::Green => &GREEN,
        TrafficColor::Yellow => &YELLOW,
        TrafficColor::Red => &RED,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "unit-test module: the fixtures cannot fail, so an unwrap failure is the intended failure signal"
)]
mod tests {
    use super::*;

    #[test]
    fn unconfigured_channel_admits_all() {
        let b = InMemoryBudget::new();
        let r = b.try_consume(99, TrafficColor::Red, 1_000);
        assert_eq!(r, ConsumeOutcome::OwnBucket);
    }

    #[test]
    fn green_takes_from_own_bucket() {
        let b = InMemoryBudget::new();
        b.configure(1, 100, 100_000, ChannelReservation::default());
        let r = b.try_consume(1, TrafficColor::Green, 100);
        assert_eq!(r, ConsumeOutcome::OwnBucket);
    }

    #[test]
    fn yellow_borrows_from_green_when_own_empty() {
        let b = InMemoryBudget::new();
        // Yellow share = 1 RPM, 100 TPM — drained by first call.
        b.configure(
            1,
            10,
            1_000,
            ChannelReservation {
                green: 0.5,
                yellow: 0.1,
                red: 0.4,
            },
        );
        // Drain Yellow (1 RPM @ 0.1 share of 10 = 1 token).
        let _ = b.try_consume(1, TrafficColor::Yellow, 50);
        // Next Yellow request → Yellow empty, must borrow Green.
        let r = b.try_consume(1, TrafficColor::Yellow, 50);
        assert_eq!(
            r,
            ConsumeOutcome::Borrowed {
                from: TrafficColor::Green,
                tpm: 50,
            }
        );
    }

    #[test]
    fn red_borrows_yellow_then_green() {
        let b = InMemoryBudget::new();
        // Red has 0 share, must borrow.
        b.configure(
            1,
            10,
            1_000,
            ChannelReservation {
                green: 0.5,
                yellow: 0.5,
                red: 0.0,
            },
        );
        let r = b.try_consume(1, TrafficColor::Red, 50);
        assert_eq!(
            r,
            ConsumeOutcome::Borrowed {
                from: TrafficColor::Yellow,
                tpm: 50,
            }
        );
    }

    #[test]
    fn green_does_not_borrow_down() {
        let b = InMemoryBudget::new();
        // Green = 1 token; once gone, Green requests get Rejected even if
        // Yellow/Red have idle capacity.
        b.configure(
            1,
            10,
            1_000,
            ChannelReservation {
                green: 0.1,
                yellow: 0.5,
                red: 0.4,
            },
        );
        let _ = b.try_consume(1, TrafficColor::Green, 50);
        // Green now empty for both RPM and TPM share. Next Green → Rejected.
        let r = b.try_consume(1, TrafficColor::Green, 50);
        assert_eq!(r, ConsumeOutcome::Rejected);
    }

    #[test]
    fn rejected_when_all_colors_drained() {
        let b = InMemoryBudget::new();
        b.configure(1, 1, 100, ChannelReservation::default());
        // Drain everything.
        for _ in 0..10 {
            let _ = b.try_consume(1, TrafficColor::Red, 100);
        }
        let r = b.try_consume(1, TrafficColor::Red, 100);
        assert_eq!(r, ConsumeOutcome::Rejected);
        assert!(!r.admitted());
    }

    #[test]
    fn refund_is_capped_at_reserved() {
        let b = InMemoryBudget::new();
        b.configure(1, 10, 1_000, ChannelReservation::default());
        // Consume 100 TPM Green, refund 99999 — should cap at green reservation.
        let _ = b.try_consume(1, TrafficColor::Green, 100);
        b.refund(1, TrafficColor::Green, 99_999);
        let snap = b.snapshot(1).unwrap();
        // Green reserved = 1000 * 0.4 = 400. Remaining must be ≤ reserved.
        assert!(snap.tpm_remaining_green <= 400);
    }

    #[test]
    fn snapshot_reflects_consumption() {
        let b = InMemoryBudget::new();
        b.configure(1, 10, 1_000, ChannelReservation::default());
        let before = b.snapshot(1).unwrap();
        let _ = b.try_consume(1, TrafficColor::Green, 50);
        let after = b.snapshot(1).unwrap();
        assert!(after.rpm_remaining_green < before.rpm_remaining_green);
        assert!(after.tpm_remaining_green < before.tpm_remaining_green);
    }

    #[test]
    fn reservation_validates_sum() {
        assert!(ChannelReservation::default().is_valid());
        let bad = ChannelReservation {
            green: 0.5,
            yellow: 0.5,
            red: 0.5,
        };
        assert!(!bad.is_valid());
    }

    #[test]
    fn outcome_admitted_flag() {
        assert!(ConsumeOutcome::OwnBucket.admitted());
        assert!(ConsumeOutcome::Borrowed {
            from: TrafficColor::Green,
            tpm: 1,
        }
        .admitted());
        assert!(!ConsumeOutcome::Rejected.admitted());
    }

    #[test]
    fn sourced_names_where_the_tokens_came_from() {
        assert_eq!(
            ConsumeOutcome::OwnBucket.sourced(),
            Some(ReservationSource::Own)
        );
        assert_eq!(
            ConsumeOutcome::Borrowed {
                from: TrafficColor::Green,
                tpm: 123,
            }
            .sourced(),
            Some(ReservationSource::Borrowed {
                from: TrafficColor::Green,
                tpm: 123,
            })
        );
        assert_eq!(
            ConsumeOutcome::Rejected.sourced(),
            None,
            "a rejected request consumed nothing, so it has no source to refund to"
        );
    }

    /// The lender is made whole and the borrower is not credited with tokens it
    /// never had — on the `Drop` (cancel) path, which refunds the full estimate.
    #[test]
    fn a_cancelled_borrow_returns_the_tokens_to_the_lender() {
        let b = InMemoryBudget::new();
        b.configure(1, 100, 100_000, ChannelReservation::default());

        // Drain Yellow (40_000 of 100_000) so the next Yellow request borrows.
        let outcome = b.try_consume(1, TrafficColor::Yellow, 40_000);
        assert_eq!(outcome, ConsumeOutcome::OwnBucket);
        BudgetGuard::with_source(
            &b,
            1,
            TrafficColor::Yellow,
            40_000,
            outcome.sourced().unwrap_or(ReservationSource::Own),
        )
        .commit(40_000);

        let before = b.snapshot(1).unwrap();
        assert_eq!(before.tpm_remaining_yellow, 0);
        let green_before = before.tpm_remaining_green;

        let outcome = b.try_consume(1, TrafficColor::Yellow, 10_000);
        assert_eq!(
            outcome,
            ConsumeOutcome::Borrowed {
                from: TrafficColor::Green,
                tpm: 10_000,
            }
        );
        drop(BudgetGuard::with_source(
            &b,
            1,
            TrafficColor::Yellow,
            10_000,
            outcome.sourced().unwrap_or(ReservationSource::Own),
        ));

        let after = b.snapshot(1).unwrap();
        assert_eq!(
            after.tpm_remaining_green, green_before,
            "green lent 10_000 to a cancelled request, so green must be whole again"
        );
        assert_eq!(
            after.tpm_remaining_yellow, 0,
            "yellow was empty and lent nothing, so it must not be credited"
        );
    }

    /// The same property on the `commit` (over-estimate) path, which refunds
    /// only the difference.
    #[test]
    fn a_committed_overestimate_returns_the_unused_part_to_the_lender() {
        let b = InMemoryBudget::new();
        b.configure(1, 100, 100_000, ChannelReservation::default());

        let outcome = b.try_consume(1, TrafficColor::Yellow, 40_000);
        BudgetGuard::with_source(
            &b,
            1,
            TrafficColor::Yellow,
            40_000,
            outcome.sourced().unwrap_or(ReservationSource::Own),
        )
        .commit(40_000);

        let green_before = b.snapshot(1).unwrap().tpm_remaining_green;

        let outcome = b.try_consume(1, TrafficColor::Yellow, 10_000);
        assert!(matches!(outcome, ConsumeOutcome::Borrowed { .. }));
        BudgetGuard::with_source(
            &b,
            1,
            TrafficColor::Yellow,
            10_000,
            outcome.sourced().unwrap_or(ReservationSource::Own),
        )
        .commit(4_000);

        let after = b.snapshot(1).unwrap();
        assert_eq!(
            after.tpm_remaining_green,
            green_before - 4_000,
            "green lent 10_000 and 6_000 came back; only the 4_000 really used stays spent"
        );
        assert_eq!(
            after.tpm_remaining_yellow, 0,
            "yellow supplied nothing, so the refund must not land in yellow"
        );
    }

    /// The cap still holds: a refund can never push a bucket past its own
    /// reservation, so no capacity is invented.
    #[test]
    fn a_borrow_refund_never_exceeds_the_lenders_reservation() {
        let b = InMemoryBudget::new();
        b.configure(1, 100, 100_000, ChannelReservation::default());

        let outcome = b.try_consume(1, TrafficColor::Yellow, 40_000);
        BudgetGuard::with_source(
            &b,
            1,
            TrafficColor::Yellow,
            40_000,
            outcome.sourced().unwrap_or(ReservationSource::Own),
        )
        .commit(40_000);

        // Borrow from green, then release far more than was ever borrowed.
        let outcome = b.try_consume(1, TrafficColor::Yellow, 10_000);
        assert!(matches!(outcome, ConsumeOutcome::Borrowed { .. }));
        BudgetGuard::with_source(
            &b,
            1,
            TrafficColor::Yellow,
            10_000,
            outcome.sourced().unwrap_or(ReservationSource::Own),
        )
        .commit(0);

        let after = b.snapshot(1).unwrap();
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
}
