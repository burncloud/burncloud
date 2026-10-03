use serde::{Deserialize, Serialize};
use std::fmt;

/// DiffServ-style three-color traffic class for the L2 Shaper.
///
/// `Green` / `Yellow` / `Red` map to 3-tier reservation buckets per channel.
/// Higher-priority colors are preferred; lower-priority colors may borrow
/// idle capacity upward. See `crates/traffic/router/src/rate_budget.rs`.
///
/// Ownership: Identity supplies the identity fact (the caller's tier), Traffic interprets it as a
/// scheduling class. The `as_char()` encoding is written into `router_logs.traffic_color`, so the
/// three characters are a storage compatibility promise.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrafficColor {
    /// Highest priority — matches Enterprise order type / reserved capacity.
    Green,
    /// Default priority — matches Value order type.
    #[default]
    Yellow,
    /// Lowest priority — matches Budget / best-effort traffic.
    Red,
}

impl TrafficColor {
    /// Static label for `router_logs.traffic_color` (single-character DB value).
    pub fn as_char(&self) -> char {
        match self {
            TrafficColor::Green => 'G',
            TrafficColor::Yellow => 'Y',
            TrafficColor::Red => 'R',
        }
    }
}

impl fmt::Display for TrafficColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_char())
    }
}
