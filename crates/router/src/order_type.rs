//! Order Type — translate three customer intents into scheduling behavior.
//!
//! Inspired by exchange-style order types (only the **name** is borrowed; the
//! product is a multi-supplier direct-routing platform, not an exchange).
//!
//! - [`OrderType::Budget`]      → Limit order: hard price ceiling. Pricier channels are filtered out;
//!   503 if all cheap channels are saturated (no fallback to expensive).
//! - [`OrderType::Value`]       → Stop-Limit order: target price + protective ceiling.
//!   Two-tier candidate set; tier-2 only used when tier-1 is exhausted. **Default for MVP.**
//! - [`OrderType::Enterprise`]  → Market order: stability over price.
//!   Reserved capacity + optional hedged redundancy.
//!
//! Per audit decision **D7**: Order Type filter runs **before** Affinity, so
//! `Budget` clients never get pinned to an expensive channel by historical
//! affinity. Per decision **D10**: MVP defaults all customers to `Value` until
//! Trader Class lands.

use burncloud_common::types::Channel;

/// Per-customer scheduling intent. See module-level docs.
#[derive(Debug, Clone, PartialEq)]
pub enum OrderType {
    /// Hard price ceiling. Channels above the cap are removed from the candidate
    /// pool entirely; if the remainder is empty the request is rejected with 503.
    /// `max_price_nanodollars` is the per-USD price cap (input + output combined).
    Budget {
        /// Per-(input + output) price cap in nanodollars per million tokens.
        max_price_nanodollars: i64,
    },

    /// Two-tier preference: try `target` first, fall back to anything ≤ `ceiling`.
    /// `Default` instance: target = i64::MAX (no preference), ceiling = i64::MAX.
    Value {
        /// Preferred per-token price ceiling for tier-1 candidates (nanodollars).
        target_nanodollars: i64,
        /// Hard fallback ceiling for tier-2 candidates (nanodollars).
        ceiling_nanodollars: i64,
    },

    /// Stability over price. Filtering is a no-op; downstream layers add
    /// hedged-request redundancy. `redundancy` is the number of parallel
    /// requests to fire (1 = no hedging, 2 = primary + hedge).
    Enterprise {
        /// Number of parallel requests to fire (1 = no hedging).
        redundancy: u8,
    },
}

impl Default for OrderType {
    /// MVP default per audit decision D10: every customer is `Value` with
    /// no effective price cap, behaving identically to today's scheduler.
    fn default() -> Self {
        OrderType::Value {
            target_nanodollars: i64::MAX,
            ceiling_nanodollars: i64::MAX,
        }
    }
}

impl OrderType {
    /// Build an `OrderType` from raw `router_tokens` columns.
    ///
    /// Centralizes the `(order_type VARCHAR(16), price_cap_nanodollars BIGINT)`
    /// → `OrderType` mapping so `proxy_logic`, future admin UI, and the cli
    /// configuration tool all share one definition (P4 — DRY).
    ///
    /// # Variant rules
    ///
    /// | DB row                                | Result                                    |
    /// |---------------------------------------|-------------------------------------------|
    /// | `("budget", Some(cap))`               | `Budget { max_price_nanodollars: cap }`   |
    /// | `("budget", None)`                    | `OrderType::default()` (Value)            |
    /// | `("enterprise", _)`                   | `Enterprise { redundancy: 1 }` (MVP — no hedging) |
    /// | `("value", _)` / unknown / `(None, _)`| `OrderType::default()` (Value)            |
    ///
    /// **`"budget"` without a `price_cap` is intentionally NOT promoted to
    /// `Budget { max_price_nanodollars: i64::MAX }`** — that would be a
    /// no-op filter dressed as a hard cap, and the resulting log line would
    /// claim Budget enforcement that never happens. Falling back to the
    /// honest `Value` default keeps `OrderType::as_label()` truthful for
    /// observability (`router_logs.layer_decision`).
    ///
    /// **Enterprise `redundancy=1`** is the MVP placeholder — hedging is
    /// deferred to a dedicated FU. Until then `Enterprise` behaves identically
    /// to `Value` at filter / dispatch time but stays distinguishable in logs.
    pub fn from_db_row(order_type: Option<&str>, price_cap: Option<i64>) -> Self {
        match (order_type, price_cap) {
            (Some("budget"), Some(cap)) => OrderType::Budget {
                max_price_nanodollars: cap,
            },
            (Some("enterprise"), _) => OrderType::Enterprise { redundancy: 1 },
            // ("budget", None) — contradictory config, fall back to honest default.
            // ("value", _) | (Some(_unknown), _) | (None, _) — explicit Value default.
            _ => OrderType::default(),
        }
    }

    /// Static label for logs / `router_logs.layer_decision` fields.
    pub fn as_label(&self) -> &'static str {
        match self {
            OrderType::Budget { .. } => "budget",
            OrderType::Value { .. } => "value",
            OrderType::Enterprise { .. } => "enterprise",
        }
    }

    /// Filter the candidate pool according to this order type.
    ///
    /// Returns the **retained** subset. The price function is injected so this
    /// module stays free of the `PriceCache` dependency — the caller resolves
    /// per-channel prices and passes a closure.
    ///
    /// # Per-OrderType behavior
    ///
    /// - `Budget`: drop any channel whose price > `max_price_nanodollars`.
    ///   If all channels exceed, returns empty — caller must surface 503.
    /// - `Value`: retain everything (no filtering at this stage). Tier
    ///   ranking happens in [`Self::tier_of`] for consumers that want it.
    /// - `Enterprise`: retain everything. Hedging is a separate concern
    ///   handled at the dispatch layer.
    pub fn filter_candidates<F>(
        &self,
        candidates: Vec<(Channel, i32)>,
        price_of: F,
    ) -> Vec<(Channel, i32)>
    where
        F: Fn(&Channel) -> Option<i64>,
    {
        match self {
            OrderType::Budget {
                max_price_nanodollars,
            } => candidates
                .into_iter()
                .filter(|(ch, _)| match price_of(ch) {
                    Some(p) => p <= *max_price_nanodollars,
                    // Unknown price: include only if the cap is at i64::MAX
                    // (i.e. no cap effectively); otherwise exclude.
                    None => *max_price_nanodollars == i64::MAX,
                })
                .collect(),
            OrderType::Value { .. } | OrderType::Enterprise { .. } => candidates,
        }
    }

    /// Returns the tier of a single channel under this order type.
    ///
    /// - `0` → no preference applies (treated equally)
    /// - `1` → tier-1 (preferred)
    /// - `2` → tier-2 (fallback, used only when tier-1 is empty/saturated)
    pub fn tier_of(&self, price_nanodollars: Option<i64>) -> u8 {
        match (self, price_nanodollars) {
            (
                OrderType::Value {
                    target_nanodollars,
                    ceiling_nanodollars,
                },
                Some(p),
            ) => {
                if p <= *target_nanodollars {
                    1
                } else if p <= *ceiling_nanodollars {
                    2
                } else {
                    // Above ceiling — should have been filtered out by a stricter
                    // OrderType. Treat as fallback tier 2.
                    2
                }
            }
            _ => 0,
        }
    }

    /// Hedged-request redundancy for [`OrderType::Enterprise`]. Returns 1 for
    /// other variants (no hedging).
    pub fn redundancy(&self) -> u8 {
        match self {
            OrderType::Enterprise { redundancy } => (*redundancy).max(1),
            _ => 1,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::scheduler::tests::make_channel;

    #[test]
    fn budget_filters_above_cap() {
        let cands = vec![make_channel(1, 10), make_channel(2, 10), make_channel(3, 10)];
        let order = OrderType::Budget {
            max_price_nanodollars: 1_000_000, // $0.001/M tokens
        };
        let prices = |ch: &Channel| -> Option<i64> {
            match ch.id {
                1 => Some(500_000),    // cheap → keep
                2 => Some(2_000_000),  // expensive → drop
                3 => Some(800_000),    // cheap → keep
                _ => None,
            }
        };
        let kept = order.filter_candidates(cands, prices);
        assert_eq!(kept.len(), 2);
        assert!(kept.iter().any(|(c, _)| c.id == 1));
        assert!(kept.iter().any(|(c, _)| c.id == 3));
    }

    #[test]
    fn budget_returns_empty_when_all_too_expensive() {
        let cands = vec![make_channel(1, 10)];
        let order = OrderType::Budget {
            max_price_nanodollars: 100,
        };
        let kept = order.filter_candidates(cands, |_| Some(1_000_000));
        assert!(kept.is_empty(), "Budget caller must surface 503 in this case");
    }

    #[test]
    fn value_default_keeps_all() {
        let cands = vec![make_channel(1, 10), make_channel(2, 10)];
        let order = OrderType::default();
        let kept = order.filter_candidates(cands, |_| Some(1_000_000));
        assert_eq!(kept.len(), 2);
    }

    #[test]
    fn enterprise_redundancy_is_at_least_one() {
        assert_eq!(OrderType::Enterprise { redundancy: 0 }.redundancy(), 1);
        assert_eq!(OrderType::Enterprise { redundancy: 3 }.redundancy(), 3);
        assert_eq!(OrderType::default().redundancy(), 1);
    }

    #[test]
    fn value_tier_classification() {
        let order = OrderType::Value {
            target_nanodollars: 1_000_000,
            ceiling_nanodollars: 5_000_000,
        };
        assert_eq!(order.tier_of(Some(500_000)), 1, "below target → tier 1");
        assert_eq!(order.tier_of(Some(3_000_000)), 2, "between target and ceiling → tier 2");
        assert_eq!(order.tier_of(Some(10_000_000)), 2, "above ceiling → tier 2 (defensive)");
        assert_eq!(order.tier_of(None), 0, "unknown price → tier 0");
    }

    #[test]
    fn budget_with_no_cap_includes_unknown_priced_channels() {
        let cands = vec![make_channel(1, 10)];
        let order = OrderType::Budget {
            max_price_nanodollars: i64::MAX,
        };
        let kept = order.filter_candidates(cands, |_| None);
        assert_eq!(kept.len(), 1);
    }
}
