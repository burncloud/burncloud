//! Passthrough scheduler — uses admin-configured weights only.

use std::collections::HashMap;

use burncloud_common::types::Channel;

use super::{ChannelScheduler, ScheduleError, SchedulingContext};

/// Passthrough scheduler — uses admin-configured weights only.
///
/// Implements `ChannelScheduler` for use in tests and as the fallback/error-recovery
/// strategy. The hot-path uses `rank_passthrough` directly to avoid HashMap allocation.
#[allow(dead_code)]
pub struct PassthroughScheduler;

impl ChannelScheduler for PassthroughScheduler {
    fn name(&self) -> &'static str {
        "passthrough"
    }

    fn score(
        &self,
        candidates: &[(Channel, i32)],
        _ctx: &SchedulingContext,
    ) -> Result<HashMap<i32, f64>, ScheduleError> {
        Ok(candidates
            .iter()
            .map(|(ch, w)| (ch.id, (*w).max(1) as f64))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::tests::make_channel;
    use std::collections::HashMap;

    fn empty_ctx() -> SchedulingContext {
        SchedulingContext {
            model: "test".into(),
            group: "default".into(),
            health_scores: HashMap::new(),
            prices: HashMap::new(),
            adaptive_limits: HashMap::new(),
            usd_cny_rate: 7.0,
        }
    }

    #[test]
    fn test_passthrough_uses_weight() {
        let candidates = vec![make_channel(1, 5), make_channel(2, 10)];
        let scores = PassthroughScheduler
            .score(&candidates, &empty_ctx())
            .unwrap();
        assert_eq!(scores[&1], 5.0);
        assert_eq!(scores[&2], 10.0);
    }

    #[test]
    fn test_passthrough_zero_weight_clamped() {
        let candidates = vec![make_channel(1, 0)];
        let scores = PassthroughScheduler
            .score(&candidates, &empty_ctx())
            .unwrap();
        assert_eq!(scores[&1], 1.0);
    }
}
