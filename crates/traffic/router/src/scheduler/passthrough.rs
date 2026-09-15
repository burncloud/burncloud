//! Passthrough scheduler — test-only implementation.
//!
//! The production passthrough path uses `rank_passthrough` directly (no HashMap allocation).
//! This module provides a `ChannelScheduler` implementation for testing purposes only.

use std::collections::HashMap;

use burncloud_common::types::Channel;

use super::{ChannelScheduler, ScheduleError, SchedulingContext};

/// Passthrough scheduler — uses admin-configured weights only.
///
/// Test-only. Production code uses `rank_passthrough` directly.
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

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::tests::make_channel;

    fn empty_ctx() -> SchedulingContext {
        SchedulingContext {
            factors: HashMap::new(),
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
