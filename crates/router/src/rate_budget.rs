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

use burncloud_common::types::TrafficColor;
use dashmap::DashMap;

/// Result of a [`BudgetBackend::try_consume`] attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumeOutcome {
    /// Tokens taken from the request's own color bucket.
    OwnBucket,
    /// Tokens taken from a higher-priority color's idle reservation.
    Borrowed { from: TrafficColor },
    /// All buckets that this color may draw from are empty.
    /// Caller should reject the request with 503 + `X-Rejected-By: shaper`.
    Rejected,
}

impl ConsumeOutcome {
    /// `true` for `OwnBucket` and `Borrowed`, `false` for `Rejected`.
    pub fn admitted(&self) -> bool {
        !matches!(self, ConsumeOutcome::Rejected)
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
        (0.99..=1.01).contains(&sum)
            && self.green >= 0.0
            && self.yellow >= 0.0
            && self.red >= 0.0
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
    fn try_consume(
        &self,
        channel_id: i32,
        color: TrafficColor,
        est_tpm: u64,
    ) -> ConsumeOutcome;

    /// Refund (return tokens to the bucket). Used after the response when the
    /// estimated TPM was higher than the actual usage — read `router_logs.cost`
    /// or token counters and call `refund(channel, color, est - actual)`.
    fn refund(&self, channel_id: i32, color: TrafficColor, tpm_to_return: u64);

    /// Read-only snapshot of remaining capacity per color (for `/router/status`).
    fn snapshot(&self, channel_id: i32) -> Option<BudgetSnapshot>;
}

/// RAII guard that ensures the TPM reservation taken by [`BudgetBackend::try_consume`]
/// is returned even if the request never reaches `commit` — i.e. on client
/// cancellation, upstream timeout, or panic during the proxy `.await`.
///
/// **Lifecycle:**
/// - `BudgetGuard::new(...)` records the `(channel_id, color, est_tpm)` triple
///   right after a successful `try_consume` (caller already holds the bucket).
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
    committed: bool,
}

impl<'a> BudgetGuard<'a> {
    /// Wrap a freshly-consumed reservation. Call after `try_consume` returned
    /// `OwnBucket` or `Borrowed` (do NOT call after `Rejected`).
    pub fn new(
        backend: &'a (dyn BudgetBackend + Send + Sync),
        channel_id: i32,
        color: TrafficColor,
        est_tpm: u64,
    ) -> Self {
        Self {
            backend,
            channel_id,
            color,
            est_tpm,
            committed: false,
        }
    }

    /// Commit the reservation with the actual TPM consumed. If the actual was
    /// smaller than the estimate, refund the difference. Marks the guard as
    /// committed so `Drop` is a no-op.
    ///
    /// Takes `self` by value — the guard is consumed and cannot be reused.
    pub fn commit(mut self, actual_tpm: u64) {
        let to_refund = self.est_tpm.saturating_sub(actual_tpm);
        if to_refund > 0 {
            self.backend
                .refund(self.channel_id, self.color, to_refund);
        }
        self.committed = true;
        // self drops here — Drop sees `committed = true` and skips full refund.
    }
}

impl<'a> Drop for BudgetGuard<'a> {
    fn drop(&mut self) {
        if !self.committed && self.est_tpm > 0 {
            // Cancel / panic / early-return path: nothing was committed by the
            // caller, so refund the full estimate. Otherwise `est_tpm` would
            // be permanently held by a request the upstream never finished.
            self.backend
                .refund(self.channel_id, self.color, self.est_tpm);
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
    fn try_consume(
        &self,
        channel_id: i32,
        color: TrafficColor,
        est_tpm: u64,
    ) -> ConsumeOutcome {
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
#[allow(clippy::unwrap_used)]
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
                from: TrafficColor::Green
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
                from: TrafficColor::Yellow
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
            from: TrafficColor::Green
        }
        .admitted());
        assert!(!ConsumeOutcome::Rejected.admitted());
    }
}
