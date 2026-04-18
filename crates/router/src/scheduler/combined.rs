//! Combined scheduler — multi-factor geometric mean scoring.
//!
//! Score = admin_weight × (health_norm^w_h × cost_norm^w_c × rpm_norm^w_r)
//!
//! Uses 0.5-offset min-max normalization to avoid extreme distortion
//! with small candidate sets.

use std::collections::HashMap;

use burncloud_common::types::Channel;

use super::{ChannelScheduler, ScheduleError, SchedulerPolicyConfig, SchedulingContext, COLD_START_RPM_LIMIT};

/// Small epsilon to avoid division by zero.
const EPS: f64 = 1e-6;

/// Per-candidate raw factor data, collected in a single pass.
struct CandidateFactors {
    health: f64,
    cost: f64,
    rpm: f64,
}

pub struct CombinedScheduler {
    config: SchedulerPolicyConfig,
}

impl CombinedScheduler {
    pub fn new(config: SchedulerPolicyConfig) -> Self {
        Self { config }
    }
}

impl ChannelScheduler for CombinedScheduler {
    fn name(&self) -> &'static str {
        "combined"
    }

    fn score(
        &self,
        candidates: &[(Channel, i32)],
        ctx: &SchedulingContext,
    ) -> Result<HashMap<i32, f64>, ScheduleError> {
        let n = candidates.len();
        if n == 0 {
            return Ok(HashMap::new());
        }

        // Single pass: collect all factors into one Vec + track min/max
        let mut factors: Vec<CandidateFactors> = Vec::with_capacity(n);
        let (mut h_min, mut h_max) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut c_min, mut c_max) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut r_min, mut r_max) = (f64::INFINITY, f64::NEG_INFINITY);

        for (ch, _) in candidates {
            let health = ctx
                .health_scores
                .get(&ch.id)
                .copied()
                .unwrap_or(1.0)
                .max(0.0);
            let health = if health.is_nan() { 1.0 } else { health };
            h_min = h_min.min(health); h_max = h_max.max(health);

            let cost = compute_cost_factor(ch, ctx);
            let cost = if cost.is_finite() && cost > 0.0 { cost } else { 1.0 };
            c_min = c_min.min(cost); c_max = c_max.max(cost);

            let rpm = rpm_factor(ch.id, ctx);
            let rpm = if rpm.is_nan() { COLD_START_RPM_LIMIT as f64 } else { rpm };
            r_min = r_min.min(rpm); r_max = r_max.max(rpm);

            factors.push(CandidateFactors { health, cost, rpm });
        }

        // Degeneracy from pre-tracked bounds
        let h_degen = (h_max - h_min).abs() < EPS;
        let c_degen = (c_max - c_min).abs() < EPS;
        let r_degen = (r_max - r_min).abs() < EPS;

        let (w_h, w_c, w_r) = self.effective_weights(h_degen, c_degen, r_degen);

        // Pre-compute normalization ranges (avoids division per candidate)
        let h_range = if h_degen { 0.0 } else { h_max - h_min };
        let c_range = if c_degen { 0.0 } else { c_max - c_min };
        let r_range = if r_degen { 0.0 } else { r_max - r_min };

        // Compute final scores with inline normalization (no HashMap lookups)
        let mut scores = HashMap::with_capacity(n);
        for ((ch, admin_w), f) in candidates.iter().zip(factors.iter()) {
            let h = if h_degen { 0.75 } else { (0.5 + 0.5 * (f.health - h_min) / h_range).clamp(0.5, 1.0) };
            let c = if c_degen { 0.75 } else { (0.5 + 0.5 * (f.cost - c_min) / c_range).clamp(0.5, 1.0) };
            let r = if r_degen { 0.75 } else { (0.5 + 0.5 * (f.rpm - r_min) / r_range).clamp(0.5, 1.0) };

            let quality = h.powf(w_h) * c.powf(w_c) * r.powf(w_r);
            let final_score = (*admin_w).max(1) as f64 * quality;

            let score = if final_score.is_finite() && final_score > 0.0 {
                final_score
            } else {
                0.0
            };
            scores.insert(ch.id, score);

            tracing::trace!(
                channel_id = ch.id,
                channel_name = %ch.name,
                admin_weight = *admin_w,
                health_norm = h,
                cost_norm = c,
                rpm_norm = r,
                effective_w_h = w_h,
                effective_w_c = w_c,
                effective_w_r = w_r,
                quality = quality,
                final_score = score,
                "scheduler scored channel"
            );
        }

        Ok(scores)
    }
}

impl CombinedScheduler {
    /// Compute effective weights, redistributing weight from degenerate dimensions.
    fn effective_weights(&self, h_degen: bool, c_degen: bool, r_degen: bool) -> (f64, f64, f64) {
        let mut w_h = if h_degen { 0.0 } else { self.config.health_weight };
        let mut w_c = if c_degen { 0.0 } else { self.config.cost_weight };
        let mut w_r = if r_degen { 0.0 } else { self.config.rpm_weight };

        // If all degenerate, fall back to equal weights (1/3 each)
        let total = w_h + w_c + w_r;
        if total <= 0.0 {
            return (1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0);
        }

        // Normalize to sum to 1.0
        w_h /= total;
        w_c /= total;
        w_r /= total;

        (w_h, w_c, w_r)
    }
}

/// Compute cost factor for a channel (lower cost = higher factor).
///
/// Looks up price from regional prices and normalizes to USD using exchange rate.
/// Without normalization, CNY-denominated prices (larger numbers) would be
/// unfairly penalized compared to USD prices.
fn compute_cost_factor(ch: &Channel, ctx: &SchedulingContext) -> f64 {
    let region = ch.pricing_region.as_deref().unwrap_or("");
    let price_raw = ctx.prices.get(region).copied().unwrap_or(0.0);

    if price_raw <= 0.0 {
        return 1.0; // Free or unknown = best
    }

    // Normalize to USD: CNY-region prices are divided by USD→CNY rate
    let is_cny_region = region.eq_ignore_ascii_case("cn") || region.eq_ignore_ascii_case("cny");
    let price_usd = if is_cny_region && ctx.usd_cny_rate > 0.0 {
        price_raw / ctx.usd_cny_rate
    } else {
        price_raw
    };

    1.0 / price_usd
}

/// Extract RPM factor from adaptive limit snapshot.
/// Cold-start channels (no data) use default matching AdaptiveLimitConfig::initial_limit.
fn rpm_factor(channel_id: i32, ctx: &SchedulingContext) -> f64 {
    ctx.adaptive_limits
        .get(&channel_id)
        .map(|snap| snap.current_limit as f64)
        .unwrap_or(COLD_START_RPM_LIMIT as f64)
}

/// 0.5-offset min-max normalization with pre-computed bounds.
/// Used by normalize_05 test helper; production code normalizes inline in score().
#[cfg(test)]
fn normalize_with_bounds(values: &[(i32, f64)], min_val: f64, max_val: f64) -> HashMap<i32, f64> {
    if values.is_empty() {
        return HashMap::new();
    }
    if (max_val - min_val).abs() < EPS {
        return values.iter().map(|&(id, _)| (id, 0.75)).collect();
    }
    let range = max_val - min_val;
    values
        .iter()
        .map(|&(id, v)| {
            let norm = 0.5 + 0.5 * (v - min_val) / range;
            (id, norm.clamp(0.5, 1.0))
        })
        .collect()
}

#[cfg(test)]
/// 0.5-offset min-max normalization (computes bounds internally).
/// Convenience wrapper for tests.
fn normalize_05(values: &[(i32, f64)]) -> HashMap<i32, f64> {
    if values.is_empty() {
        return HashMap::new();
    }
    let (min_val, max_val) = values.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(min, max), &(_, v)| (min.min(v), max.max(v)),
    );
    normalize_with_bounds(values, min_val, max_val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::tests::make_channel;

    fn make_ctx(
        health: HashMap<i32, f64>,
        prices: HashMap<String, f64>,
        limits: HashMap<i32, u32>,
    ) -> SchedulingContext {
        SchedulingContext {
            health_scores: health,
            prices,
            adaptive_limits: limits
                .into_iter()
                .map(|(id, lim)| {
                    (
                        id,
                        crate::adaptive_limit::AdaptiveSnapshot {
                            current_limit: lim,
                            state: crate::adaptive_limit::RateLimitState::Stable,
                        },
                    )
                })
                .collect(),
            usd_cny_rate: 7.0,
        }
    }

    #[test]
    fn test_combined_prefers_healthier() {
        let c1 = make_channel(1, 10);
        let c2 = make_channel(2, 10);
        let ctx = make_ctx(
            HashMap::from([(1, 0.9), (2, 0.5)]),
            HashMap::new(),
            HashMap::new(),
        );
        let scheduler = CombinedScheduler::new(SchedulerPolicyConfig {
            health_weight: 1.0,
            cost_weight: 0.0,
            rpm_weight: 0.0,
        });
        let scores = scheduler.score(&[c1, c2], &ctx).unwrap();
        assert!(
            scores[&1] > scores[&2],
            "healthier channel should score higher"
        );
    }

    #[test]
    fn test_combined_prefers_cheaper() {
        let mut c1 = make_channel(1, 10);
        c1.0.pricing_region = Some("cn".into());
        let mut c2 = make_channel(2, 10);
        c2.0.pricing_region = Some("us".into());

        let ctx = make_ctx(
            HashMap::new(),
            HashMap::from([("cn".to_string(), 100.0), ("us".to_string(), 500.0)]),
            HashMap::new(),
        );
        let scheduler = CombinedScheduler::new(SchedulerPolicyConfig {
            health_weight: 0.0,
            cost_weight: 1.0,
            rpm_weight: 0.0,
        });
        let scores = scheduler.score(&[c1, c2], &ctx).unwrap();
        assert!(
            scores[&1] > scores[&2],
            "cheaper channel should score higher"
        );
    }

    #[test]
    fn test_combined_prefers_higher_rpm() {
        let c1 = make_channel(1, 10);
        let c2 = make_channel(2, 10);
        let ctx = make_ctx(
            HashMap::new(),
            HashMap::new(),
            HashMap::from([(1, 100), (2, 10)]),
        );
        let scheduler = CombinedScheduler::new(SchedulerPolicyConfig {
            health_weight: 0.0,
            cost_weight: 0.0,
            rpm_weight: 1.0,
        });
        let scores = scheduler.score(&[c1, c2], &ctx).unwrap();
        assert!(
            scores[&1] > scores[&2],
            "higher RPM should score higher"
        );
    }

    #[test]
    fn test_normalize_05_two_candidates() {
        let values = vec![(1, 1.0), (2, 5.0)];
        let norm = normalize_05(&values);
        // 1.0 → 0.5, 5.0 → 1.0
        assert!((norm[&1] - 0.5).abs() < 1e-9);
        assert!((norm[&2] - 1.0).abs() < 1e-9);
        // Ratio is 2:1, not 1000:1
        assert!(norm[&2] / norm[&1] < 3.0);
    }

    #[test]
    fn test_normalize_05_degenerate() {
        let values = vec![(1, 3.0), (2, 3.0), (3, 3.0)];
        let norm = normalize_05(&values);
        for id in [1, 2, 3] {
            assert!((norm[&id] - 0.75).abs() < 1e-9);
        }
    }

    #[test]
    fn test_zero_price_gets_best_score() {
        let mut c1 = make_channel(1, 10);
        c1.0.pricing_region = Some("free".into());
        let mut c2 = make_channel(2, 10);
        c2.0.pricing_region = Some("paid".into());

        let ctx = make_ctx(
            HashMap::new(),
            HashMap::from([("free".to_string(), 0.0), ("paid".to_string(), 100.0)]),
            HashMap::new(),
        );
        let scheduler = CombinedScheduler::new(SchedulerPolicyConfig {
            health_weight: 0.0,
            cost_weight: 1.0,
            rpm_weight: 0.0,
        });
        let scores = scheduler.score(&[c1, c2], &ctx).unwrap();
        assert!(
            scores[&1] > scores[&2],
            "free channel should score higher"
        );
    }

    #[test]
    fn test_cold_start_rpm_default() {
        // No adaptive_limits entries → cold start
        let c1 = make_channel(1, 10);
        let c2 = make_channel(2, 10);
        let ctx = make_ctx(HashMap::new(), HashMap::new(), HashMap::new());
        let scheduler = CombinedScheduler::new(SchedulerPolicyConfig {
            health_weight: 0.0,
            cost_weight: 0.0,
            rpm_weight: 1.0,
        });
        let scores = scheduler.score(&[c1, c2], &ctx).unwrap();
        // Both should get equal score (degenerate RPM → both get 10.0 default)
        assert!(
            (scores[&1] - scores[&2]).abs() < 0.01,
            "cold-start channels should get equal RPM score"
        );
    }

    #[test]
    fn test_cross_currency_cost_comparison() {
        // CN channel: price in CNY (larger number, e.g. 18 CNY/MTok)
        // US channel: price in USD (smaller number, e.g. 2.5 USD/MTok)
        // At 7.2 CNY/USD: CN = 18/7.2 = 2.5 USD, should be equal
        let mut c1 = make_channel(1, 10);
        c1.0.pricing_region = Some("cn".into());
        let mut c2 = make_channel(2, 10);
        c2.0.pricing_region = Some("us".into());

        let mut ctx = make_ctx(
            HashMap::new(),
            HashMap::from([("cn".to_string(), 18.0), ("us".to_string(), 2.5)]),
            HashMap::new(),
        );
        ctx.usd_cny_rate = 7.2;

        let scheduler = CombinedScheduler::new(SchedulerPolicyConfig {
            health_weight: 0.0,
            cost_weight: 1.0,
            rpm_weight: 0.0,
        });
        let scores = scheduler.score(&[c1, c2], &ctx).unwrap();
        // After CNY→USD normalization, both should have equal cost
        let ratio = scores[&1] / scores[&2];
        assert!(
            (ratio - 1.0).abs() < 0.1,
            "cross-currency equal cost should give similar scores, got ratio {ratio}"
        );
    }
}
