//! 用户组的多供给路由策略。
//!
//! 未配置策略时默认使用 `balanced`；三种新策略只在最高可用管理员优先级内选择：
//! - `balanced`：按正管理员权重分摊。
//! - `stability_first`：优先最高有限健康分，同分按权重分摊。
//! - `price_first`：优先最低可比较报价，同价按权重分摊。
//!
//! 显式 `passthrough` 和 `combined` 保留历史排序、会话亲和与候选截断行为。

mod combined;
mod strict;
pub(crate) use strict::StrictPlan;
#[cfg(test)]
mod passthrough;

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::str::FromStr;

use burncloud_commerce_billing::PriceCache;
use burncloud_supply_contracts::Channel;
use burncloud_traffic_contracts::TrafficColor;
use serde::{Deserialize, Serialize};

use crate::channel_state::ChannelStateTracker;
use crate::exchange_rate::ExchangeRateService;
use crate::order_type::OrderType;

pub(crate) use combined::CombinedScheduler;
#[cfg(test)]
pub(crate) use passthrough::PassthroughScheduler;

/// Error type for scheduling operations.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ScheduleError {
    #[error("scheduling failed: {0}")]
    #[allow(
        dead_code,
        reason = "constructed by trait implementors; the compiler cannot see across dyn dispatch"
    )]
    Internal(String),
}

/// Pre-computed scheduling factors for a single candidate.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CandidateFactors {
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
pub(crate) struct SchedulingContext {
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
        self.session_id.as_deref().or(self.user_id.as_deref())
    }
}

/// Trait for channel scheduling strategies.
///
/// Implementations must be stateless and panic-safe.
/// The orchestrator wraps `score()` in `catch_unwind` for protection.
pub(crate) trait ChannelScheduler: Send + Sync {
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
        weights
            .iter()
            .all(|w| *w >= 0.0 && w.is_finite() && !w.is_nan())
            && weights.iter().any(|w| *w > 0.0)
    }
}

/// 用户组的路由策略；未配置时默认均衡，显式旧策略保留旧行为。
///
/// # Examples
/// ```
/// use burncloud_router::SchedulerKind;
/// let policy = SchedulerKind::Balanced;
/// ```
///
/// # Notes
/// 三种新策略只在最高可用管理员档内选择，不受会话亲和覆盖。
/// 新增变体要求外部穷尽 match 同步更新。
#[derive(Debug, Clone)]
pub enum SchedulerKind {
    /// 按正管理员权重分摊。
    Balanced,
    /// 最高有限健康分，同分按权重。
    StabilityFirst,
    /// 最低可比较 nano-USD 参考价，同价按权重。
    PriceFirst,
    /// 显式保留历史评分、亲和与候选截断行为。
    Passthrough,
    /// 显式保留历史健康/成本/RPM 综合评分行为。
    Combined { config: SchedulerPolicyConfig },
}

/// Maps group name → scheduler kind.
pub(crate) type SchedulerPolicyMap = HashMap<String, SchedulerKind>;

/// RPM factor for channels in Learning state — neutral, capacity unknown.
const RPM_FACTOR_LEARNING: f64 = 1.0;
/// RPM factor for channels in Cooldown state — severely penalized.
const RPM_FACTOR_COOLDOWN: f64 = 0.1;

impl SchedulerKind {
    pub(crate) fn is_strict(&self) -> bool {
        matches!(
            self,
            Self::Balanced | Self::StabilityFirst | Self::PriceFirst
        )
    }
}

pub(crate) fn load_scheduler_config() -> anyhow::Result<SchedulerPolicyMap> {
    match std::env::var("SCHEDULER_POLICIES") {
        Ok(raw) => parse_scheduler_config(&raw),
        Err(std::env::VarError::NotPresent) => Ok(HashMap::new()),
        Err(error) => Err(anyhow::anyhow!("SCHEDULER_POLICIES: {error}")),
    }
}

fn parse_scheduler_config(raw: &str) -> anyhow::Result<SchedulerPolicyMap> {
    // 自定义 map visitor 保留并拒绝重复键，不能先收集为 Value 而丢掉重复证据。
    struct Entries(Vec<(String, SchedulerPolicyEntry)>);
    impl<'de> Deserialize<'de> for Entries {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct Visitor;
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = Entries;
                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str("scheduler group map")
                }
                fn visit_map<M: serde::de::MapAccess<'de>>(
                    self,
                    mut map: M,
                ) -> Result<Entries, M::Error> {
                    let mut entries = Vec::new();
                    while let Some(entry) = map.next_entry()? {
                        entries.push(entry);
                    }
                    Ok(Entries(entries))
                }
            }
            deserializer.deserialize_map(Visitor)
        }
    }
    let entries: Entries = serde_json::from_str(raw)
        .map_err(|e| anyhow::anyhow!("invalid SCHEDULER_POLICIES: {e}"))?;
    let mut result = HashMap::new();
    for (group, entry) in entries.0 {
        let name = group.to_lowercase();
        anyhow::ensure!(
            !name.trim().is_empty() && !result.contains_key(&name),
            "invalid or duplicate scheduler group: {group}"
        );
        if entry.scheduler_type != "combined" {
            anyhow::ensure!(
                entry.health_weight.is_none()
                    && entry.cost_weight.is_none()
                    && entry.rpm_weight.is_none(),
                "unexpected weights for scheduler group: {group}"
            );
        }
        let kind = match entry.scheduler_type.as_str() {
            "balanced" => SchedulerKind::Balanced,
            "stability_first" => SchedulerKind::StabilityFirst,
            "price_first" => SchedulerKind::PriceFirst,
            "passthrough" => SchedulerKind::Passthrough,
            "combined" => {
                let config = SchedulerPolicyConfig {
                    health_weight: entry.health_weight.unwrap_or_else(default_health_weight),
                    cost_weight: entry.cost_weight.unwrap_or_else(default_cost_weight),
                    rpm_weight: entry.rpm_weight.unwrap_or_else(default_rpm_weight),
                };
                anyhow::ensure!(
                    config.validate(),
                    "invalid scheduler weights for group: {group}"
                );
                SchedulerKind::Combined { config }
            }
            other => anyhow::bail!("unknown scheduler type: {other}"),
        };
        result.insert(name, kind);
    }
    Ok(result)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SchedulerPolicyEntry {
    #[serde(rename = "type")]
    scheduler_type: String,
    health_weight: Option<f64>,
    cost_weight: Option<f64>,
    rpm_weight: Option<f64>,
}

/// Pick the scheduler for a group (case-insensitive, falls back to Passthrough).
///
/// Currently only used in tests; `route_with_scheduler` inlines this logic
/// to avoid an extra dyn dispatch when the passthrough fast-path is taken.
#[cfg(test)]
pub(crate) fn pick_scheduler<'a>(
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
pub(crate) fn rank_candidates(
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
            tracing::warn!(
                "Scheduler '{}' returned error: {e}, falling back to passthrough",
                scheduler.name()
            );
            return rank_passthrough(candidates);
        }
        Err(payload) => {
            tracing::error!(
                "Scheduler '{}' panicked: {}, falling back to passthrough",
                scheduler.name(),
                payload.downcast_ref::<&str>().unwrap_or(&"unknown panic")
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
pub(crate) fn rank_passthrough(mut candidates: Vec<(Channel, i32)>) -> Vec<(Channel, i32)> {
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
pub(crate) async fn build_context(
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
        lookup_region_price_usd(&mut prices_usd, region, model, price_cache, exchange_rate).await;

        // Combined health + adaptive lookup (1 DashMap get + 1 HashMap get)
        let (health, adaptive) = state_tracker.get_health_and_adaptive(ch.id, model);

        // Pre-compute cost factor (prices already USD-normalized at cache time)
        let cost = {
            let price_usd = prices_usd.get(region).copied().unwrap_or(0.0);
            if price_usd <= 0.0 {
                1.0
            } else {
                1.0 / price_usd
            }
        };

        let rpm = match adaptive.state {
            crate::aimd_limiter::RateLimitState::Stable => adaptive.current_limit as f64,
            crate::aimd_limiter::RateLimitState::Learning => RPM_FACTOR_LEARNING,
            crate::aimd_limiter::RateLimitState::Cooldown => RPM_FACTOR_COOLDOWN,
        };

        factors.insert(ch.id, CandidateFactors { health, cost, rpm });
    }

    SchedulingContext { factors }
}

/// Resolve a region's model price in USD and memoize it in `prices_usd`.
///
/// Regions are looked up once per `build_context` pass; a region whose price is
/// missing stays absent from the map so the caller uses the default cost factor.
async fn lookup_region_price_usd<'a>(
    prices_usd: &mut HashMap<&'a str, f64>,
    region: &'a str,
    model: &str,
    price_cache: &PriceCache,
    exchange_rate: &ExchangeRateService,
) {
    if prices_usd.contains_key(region) {
        return;
    }

    let region_filter = if region.is_empty() {
        None
    } else {
        Some(region)
    };
    let Some(price) = price_cache.get(model, region_filter).await else {
        if !region.is_empty() {
            tracing::debug!(
                "No price data for model='{model}' region='{region}', cost factor will use default"
            );
        }
        return;
    };

    let price_usd = normalized_price_usd(
        price.input_price,
        price.output_price,
        &price.currency,
        exchange_rate,
    )
    .await;

    // The entry was confirmed vacant above; insertion cannot overwrite a price.
    if let std::collections::hash_map::Entry::Vacant(e) = prices_usd.entry(region) {
        e.insert(price_usd);
    }
}

/// Sum a price's input/output components and normalize the total to USD.
async fn normalized_price_usd(
    input_price: i64,
    output_price: i64,
    currency: &str,
    exchange_rate: &ExchangeRateService,
) -> f64 {
    use burncloud_commerce_contracts::pricing::Currency;

    let raw = input_price as f64 + output_price as f64;
    match Currency::from_str(currency) {
        Ok(curr) if curr != Currency::USD => {
            match exchange_rate.convert(raw, curr, Currency::USD) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Currency conversion failed: {e}, using raw amount");
                    raw
                }
            }
        }
        _ => raw,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Helper to create a Channel for tests.
    pub(crate) fn make_channel(id: i32, weight: i32) -> (Channel, i32) {
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
        assert_eq!(
            result[0].0.id, 2,
            "fallback should order by weight (10 > 5)"
        );
        assert_eq!(result[1].0.id, 1);
    }

    /// A scheduler that panics, to test catch_unwind fallback.
    struct PanickingScheduler;

    impl ChannelScheduler for PanickingScheduler {
        fn name(&self) -> &'static str {
            "panicking"
        }
        #[allow(
            clippy::panic_in_result_fn,
            reason = "test fixture: the panic is the behavior under test (catch_unwind fallback)"
        )]
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
