//! Channel Scheduling Strategy Engine
//!
//! Provides a trait-based scheduling system for multi-channel model routing.
//! Supports two modes:
//! - **Passthrough**: Uses admin-configured weights only (default, backward-compatible)
//! - **Combined**: Multi-factor scoring using health, cost, and RPM rate limits

mod combined;
#[cfg(test)]
mod passthrough;

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::str::FromStr;

use burncloud_common::types::{Channel, TrafficColor};
use burncloud_service_billing::PriceCache;
use serde::{Deserialize, Serialize};

use crate::channel_state::ChannelStateTracker;
use crate::exchange_rate::ExchangeRateService;
use crate::order_type::OrderType;

pub use combined::CombinedScheduler;
#[cfg(test)]
pub use passthrough::PassthroughScheduler;

/// Error type for scheduling operations.
#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    #[error("scheduling failed: {0}")]
    #[allow(dead_code)] // Used by trait implementors; compiler can't see across dyn dispatch
    Internal(String),
}

/// Pre-computed scheduling factors for a single candidate.
#[derive(Debug, Clone, Copy)]
pub struct CandidateFactors {
    pub health: f64,
    pub cost: f64,
    pub rpm: f64,
}

/// Read-only context assembled per scheduling decision.
///
/// Contains **candidate-dimension** snapshots only (one entry per channel).
/// Request-dimension fields (user_id, color, order_type) live in
/// [`SchedulingRequest`] and are NOT visible to scoring strategies — the
/// DiffServ principle: schedulers stay color-agnostic, the L2 Shaper is the
/// only layer that consumes color (audit decision D6 / E-D1 / E-D2).
#[derive(Debug, Clone, Default)]
pub struct SchedulingContext {
    /// Per-candidate pre-computed factors (health, cost, rpm).
    pub factors: HashMap<i32, CandidateFactors>,
}

/// Per-request metadata flowing from Server → Router.
///
/// Carries the L1 Classifier output (color + order type) and the user
/// identity used for L3 Affinity (HRW key). Scorers (L4) never see this
/// struct — they only get [`SchedulingContext`].
#[derive(Debug, Clone, Default)]
pub struct SchedulingRequest {
    /// User identifier — affinity key for L3 (HRW). `None` disables affinity.
    pub user_id: Option<String>,
    /// DiffServ color classification (L1 → L2 Shaper).
    pub color: TrafficColor,
    /// Order Type (Budget / Value / Enterprise) — L1 → L3 candidate filter.
    pub order_type: OrderType,
    /// Optional conversation/session id; if present, takes precedence over
    /// `user_id` as the affinity key (preserves KV cache across same-conversation
    /// turns even when shared across users — see Part 4 取舍 5).
    pub session_id: Option<String>,
}

impl SchedulingRequest {
    /// Returns the affinity key — session_id when present, else user_id.
    pub fn affinity_key(&self) -> Option<&str> {
        self.session_id
            .as_deref()
            .or(self.user_id.as_deref())
    }
}

/// Trait for channel scheduling strategies.
///
/// Implementations must be stateless and panic-safe.
/// The orchestrator wraps `score()` in `catch_unwind` for protection.
pub trait ChannelScheduler: Send + Sync {
    fn name(&self) -> &'static str;
    fn score(
        &self,
        candidates: &[(Channel, i32)],
        ctx: &SchedulingContext,
    ) -> Result<HashMap<i32, f64>, ScheduleError>;
}

/// Configuration for the combined (multi-factor) scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerPolicyConfig {
    #[serde(default = "default_health_weight")]
    pub health_weight: f64,
    #[serde(default = "default_cost_weight")]
    pub cost_weight: f64,
    #[serde(default = "default_rpm_weight")]
    pub rpm_weight: f64,
}

fn default_health_weight() -> f64 {
    0.4
}
fn default_cost_weight() -> f64 {
    0.4
}
fn default_rpm_weight() -> f64 {
    0.2
}

impl Default for SchedulerPolicyConfig {
    fn default() -> Self {
        Self {
            health_weight: default_health_weight(),
            cost_weight: default_cost_weight(),
            rpm_weight: default_rpm_weight(),
        }
    }
}

impl SchedulerPolicyConfig {
    /// Validate weights: must be non-negative, finite, and at least one positive.
    pub fn validate(&self) -> bool {
        let weights = [self.health_weight, self.cost_weight, self.rpm_weight];
        weights.iter().all(|w| *w >= 0.0 && w.is_finite() && !w.is_nan())
            && weights.iter().any(|w| *w > 0.0)
    }
}

/// Selects which scheduler to use for a group.
#[derive(Debug, Clone)]
pub enum SchedulerKind {
    Passthrough,
    Combined { config: SchedulerPolicyConfig },
}

/// Maps group name → scheduler kind.
pub type SchedulerPolicyMap = HashMap<String, SchedulerKind>;

/// RPM factor for channels in Learning state — neutral, capacity unknown.
const RPM_FACTOR_LEARNING: f64 = 1.0;
/// RPM factor for channels in Cooldown state — severely penalized.
const RPM_FACTOR_COOLDOWN: f64 = 0.1;

/// Load scheduler policies from environment configuration.
///
/// Reads `SCHEDULER_POLICIES` env var (JSON) with format:
/// ```json
/// {
///   "vip": { "type": "combined", "health_weight": 0.4, "cost_weight": 0.4, "rpm_weight": 0.2 },
///   "default": { "type": "passthrough" }
/// }
/// ```
///
/// Falls back to all-groups-passthrough if env var is missing or invalid.
pub fn load_scheduler_config() -> SchedulerPolicyMap {
    let json_str = match std::env::var("SCHEDULER_POLICIES") {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };

    let raw: HashMap<String, serde_json::Value> = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Failed to parse SCHEDULER_POLICIES: {e}");
            return HashMap::new();
        }
    };

    let mut policies = HashMap::new();
    for (group, val) in raw {
        let kind = match serde_json::from_value::<SchedulerPolicyEntry>(val) {
            Ok(entry) => match entry.scheduler_type.as_str() {
                "combined" => {
                    let config = SchedulerPolicyConfig {
                        health_weight: entry.health_weight.unwrap_or_else(default_health_weight),
                        cost_weight: entry.cost_weight.unwrap_or_else(default_cost_weight),
                        rpm_weight: entry.rpm_weight.unwrap_or_else(default_rpm_weight),
                    };
                    if !config.validate() {
                        tracing::warn!(
                            "Invalid scheduler weights for group '{}', falling back to passthrough",
                            group
                        );
                        SchedulerKind::Passthrough
                    } else {
                        SchedulerKind::Combined { config }
                    }
                }
                _ => SchedulerKind::Passthrough,
            },
            Err(e) => {
                tracing::warn!("Failed to parse scheduler entry for group '{}': {e}", group);
                SchedulerKind::Passthrough
            }
        };
        policies.insert(group.to_lowercase(), kind);
    }

    tracing::info!("Loaded {} scheduler policies", policies.len());
    policies
}

#[derive(Deserialize)]
struct SchedulerPolicyEntry {
    #[serde(rename = "type", default = "default_type")]
    scheduler_type: String,
    #[serde(default)]
    health_weight: Option<f64>,
    #[serde(default)]
    cost_weight: Option<f64>,
    #[serde(default)]
    rpm_weight: Option<f64>,
}

fn default_type() -> String {
    "passthrough".to_string()
}

/// Pick the scheduler for a group (case-insensitive, falls back to Passthrough).
///
/// Currently only used in tests; `route_with_scheduler` inlines this logic
/// to avoid an extra dyn dispatch when the passthrough fast-path is taken.
#[cfg(test)]
pub fn pick_scheduler<'a>(
    group: &str,
    policies: &'a SchedulerPolicyMap,
    passthrough: &'a PassthroughScheduler,
    combined: &'a CombinedScheduler,
) -> &'a dyn ChannelScheduler {
    match policies.get(&group.to_lowercase()) {
        Some(SchedulerKind::Combined { .. }) => combined as &dyn ChannelScheduler,
        _ => passthrough as &dyn ChannelScheduler,
    }
}

/// Rank candidates by scheduler score, returning sorted (Channel, weight) pairs.
///
/// Wraps `score()` in `catch_unwind` for panic protection.
/// On panic or error, falls back to PassthroughScheduler ordering.
/// Takes ownership to avoid cloning Channels.
pub fn rank_candidates(
    candidates: Vec<(Channel, i32)>,
    ctx: &SchedulingContext,
    scheduler: &dyn ChannelScheduler,
) -> Vec<(Channel, i32)> {
    if candidates.len() <= 1 {
        return candidates;
    }

    let scores = match catch_unwind(AssertUnwindSafe(|| scheduler.score(&candidates, ctx))) {
        Ok(Ok(map)) => map,
        Ok(Err(e)) => {
            tracing::warn!("Scheduler '{}' returned error: {e}, falling back to passthrough", scheduler.name());
            return rank_passthrough(candidates);
        }
        Err(payload) => {
            tracing::error!(
                "Scheduler '{}' panicked: {}, falling back to passthrough",
                scheduler.name(),
                payload
                    .downcast_ref::<&str>()
                    .unwrap_or(&"unknown panic")
            );
            return rank_passthrough(candidates);
        }
    };

    // Sort candidates in-place by score (descending) — no index Vec or Option wrapping needed
    let mut candidates = candidates;
    candidates.sort_by(|a, b| {
        let sa = scores.get(&a.0.id).copied().unwrap_or(0.0);
        let sb = scores.get(&b.0.id).copied().unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    candidates
}

/// Rank candidates using PassthroughScheduler (no context needed).
///
/// Short-circuits to a simple sort by admin weight (descending) without
/// allocating SchedulingContext or going through the full scoring pipeline.
/// Takes ownership to avoid cloning Channels.
pub fn rank_passthrough(mut candidates: Vec<(Channel, i32)>) -> Vec<(Channel, i32)> {
    if candidates.len() <= 1 {
        return candidates;
    }
    candidates.sort_by_key(|b| std::cmp::Reverse(b.1));
    candidates
}

/// Assemble a SchedulingContext from live state.
///
/// Pre-computes all scheduling factors (health, cost, rpm) per candidate
/// in a single pass over channel state.
pub async fn build_context(
    model: &str,
    candidates: &[(Channel, i32)],
    state_tracker: &ChannelStateTracker,
    price_cache: &PriceCache,
    exchange_rate: &ExchangeRateService,
) -> SchedulingContext {
    // Single pass: collect prices + health + adaptive + pre-computed cost
    let mut prices_usd: HashMap<&str, f64> = HashMap::new();
    let mut factors = HashMap::with_capacity(candidates.len());

    for (ch, _) in candidates {
        // Look up price for this channel's region (deduplicated, normalized to USD)
        let region = ch.pricing_region.as_deref().unwrap_or("");
        if let std::collections::hash_map::Entry::Vacant(e) = prices_usd.entry(region) {
            if let Some(price) = price_cache.get(model, if region.is_empty() { None } else { Some(region) }).await {
                let raw = price.input_price as f64 + price.output_price as f64;
                let price_usd = match burncloud_common::Currency::from_str(&price.currency) {
                    Ok(curr) if curr != burncloud_common::Currency::USD => {
                        exchange_rate.convert(raw, curr, burncloud_common::Currency::USD)
                    }
                    _ => raw,
                };
                e.insert(price_usd);
            } else if !region.is_empty() {
                tracing::debug!("No price data for model='{model}' region='{region}', cost factor will use default");
            }
        }

        // Combined health + adaptive lookup (1 DashMap get + 1 HashMap get)
        let (health, adaptive) = state_tracker.get_health_and_adaptive(ch.id, model);

        // Pre-compute cost factor (prices already USD-normalized at cache time)
        let cost = {
            let price_usd = prices_usd.get(region).copied().unwrap_or(0.0);
            if price_usd <= 0.0 { 1.0 } else { 1.0 / price_usd }
        };

        let rpm = match adaptive.state {
            crate::aimd_limiter::RateLimitState::Stable => adaptive.current_limit as f64,
            crate::aimd_limiter::RateLimitState::Learning => RPM_FACTOR_LEARNING,
            crate::aimd_limiter::RateLimitState::Cooldown => RPM_FACTOR_COOLDOWN,
        };

        factors.insert(ch.id, CandidateFactors {
            health,
            cost,
            rpm,
        });
    }

    SchedulingContext { factors }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Helper to create a Channel for tests.
    pub fn make_channel(id: i32, weight: i32) -> (Channel, i32) {
        (
            Channel {
                id,
                type_: 1,
                key: format!("key-{id}"),
                status: 1,
                name: format!("ch-{id}"),
                weight,
                created_time: None,
                test_time: None,
                response_time: None,
                base_url: Some(format!("https://ch{id}.example.com")),
                models: String::new(),
                group: "default".to_string(),
                used_quota: 0,
                model_mapping: None,
                priority: 0,
                auto_ban: 0,
                other_info: None,
                tag: None,
                setting: None,
                param_override: None,
                header_override: None,
                remark: None,
                api_version: None,
                pricing_region: None,
                rpm_cap: None,
                tpm_cap: None,
                reservation_green: None,
                reservation_yellow: None,
                reservation_red: None,
            },
            weight,
        )
    }

    #[test]
    fn test_passthrough_default_fallback() {
        let policies: SchedulerPolicyMap = HashMap::new();
        let passthrough = PassthroughScheduler;
        let combined = CombinedScheduler::new(SchedulerPolicyConfig::default());

        let s = pick_scheduler("any", &policies, &passthrough, &combined);
        assert_eq!(s.name(), "passthrough");
    }

    #[test]
    fn test_validate_rejects_nan() {
        let config = SchedulerPolicyConfig {
            health_weight: f64::NAN,
            cost_weight: 0.4,
            rpm_weight: 0.2,
        };
        assert!(!config.validate());
    }

    #[test]
    fn test_validate_rejects_all_zero() {
        let config = SchedulerPolicyConfig {
            health_weight: 0.0,
            cost_weight: 0.0,
            rpm_weight: 0.0,
        };
        assert!(!config.validate());
    }

    #[test]
    fn test_rank_candidates_single() {
        let c = vec![make_channel(1, 10)];
        let ctx = SchedulingContext::default();
        let passthrough = PassthroughScheduler;
        let result = rank_candidates(c, &ctx, &passthrough);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.id, 1);
    }

    #[test]
    fn test_rank_passthrough_shortcut() {
        let c = vec![make_channel(1, 5), make_channel(2, 10)];
        let result = rank_passthrough(c);
        assert_eq!(result.len(), 2);
        // Higher weight should be first
        assert_eq!(result[0].0.id, 2);
    }

    /// A scheduler that always returns an error, to test fallback behavior.
    struct FailingScheduler;

    impl ChannelScheduler for FailingScheduler {
        fn name(&self) -> &'static str {
            "failing"
        }
        fn score(
            &self,
            _candidates: &[(Channel, i32)],
            _ctx: &SchedulingContext,
        ) -> Result<HashMap<i32, f64>, ScheduleError> {
            Err(ScheduleError::Internal("intentional failure".into()))
        }
    }

    #[test]
    fn test_rank_candidates_error_fallback_to_passthrough() {
        let c = vec![make_channel(1, 5), make_channel(2, 10)];
        let ctx = SchedulingContext::default();
        let failing = FailingScheduler;
        let result = rank_candidates(c, &ctx, &failing);
        // Should fall back to passthrough ordering (higher weight first)
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0.id, 2, "fallback should order by weight (10 > 5)");
        assert_eq!(result[1].0.id, 1);
    }

    /// A scheduler that panics, to test catch_unwind fallback.
    struct PanickingScheduler;

    impl ChannelScheduler for PanickingScheduler {
        fn name(&self) -> &'static str {
            "panicking"
        }
        fn score(
            &self,
            _candidates: &[(Channel, i32)],
            _ctx: &SchedulingContext,
        ) -> Result<HashMap<i32, f64>, ScheduleError> {
            panic!("intentional panic");
        }
    }

    #[test]
    fn test_rank_candidates_panic_fallback_to_passthrough() {
        let c = vec![make_channel(1, 3), make_channel(2, 7), make_channel(3, 1)];
        let ctx = SchedulingContext::default();
        let panicking = PanickingScheduler;
        let result = rank_candidates(c, &ctx, &panicking);
        // Should fall back to passthrough ordering (7, 3, 1)
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0.id, 2);
        assert_eq!(result[1].0.id, 1);
        assert_eq!(result[2].0.id, 3);
    }
}
