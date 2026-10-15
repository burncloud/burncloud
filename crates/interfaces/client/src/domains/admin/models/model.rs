//! Presentation helpers for the administrator model catalog.

use crate::domains::buyer::marketplace::model::RoutingTier;

pub(super) fn format_price(price: f64) -> String {
    format!("{price:.2}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

pub(super) const fn tier_label(tier: RoutingTier) -> &'static str {
    match tier {
        RoutingTier::Economy => "ECONOMY",
        RoutingTier::Standard => "STANDARD",
        RoutingTier::Performance => "PERFORMANCE",
    }
}
