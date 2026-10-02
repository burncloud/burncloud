//! Compatibility re-export of the Commerce-owned nanodollar conversion contract.
//!
//! The single implementation of these functions and constants now lives in
//! `burncloud_commerce_contracts::price_u64` (Commerce domain). This module keeps the
//! former `burncloud_common::price_u64` paths working for existing consumers, and
//! `burncloud_common` only re-exports here: it does not define a second implementation.
//!
//! Signatures, the i64 nanodollar unit and the rounding/truncation behavior are
//! unchanged by the move.
//!
//! # Example
//! ```
//! use burncloud_common::price_u64::{calculate_cost_safe, dollars_to_nano};
//!
//! assert_eq!(dollars_to_nano(3.0), 3_000_000_000);
//! assert_eq!(calculate_cost_safe(1_000_000, 3_000_000_000), 3_000_000_000);
//! ```

pub use burncloud_commerce_contracts::price_u64::{
    calculate_cost_safe, dollars_to_nano, nano_to_dollars, rate_to_scaled, scaled_to_rate,
    NANO_PER_DOLLAR, RATE_SCALE,
};
