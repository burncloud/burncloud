// Compatibility shim: burncloud_common::pricing_config now re-exports the Commerce-owned
// pricing document contract. The schema implementation (and its tests) moved to
// `burncloud_commerce_contracts::pricing` in S1-B; nothing is defined here any more.
//
// This module keeps `burncloud_common::pricing_config::{PricingConfig, ...}` and the
// root-level `burncloud_common::{PricingConfig, ...}` glob export working for existing
// consumers until they switch to the Commerce contract.
//
// S1 note: `ModelMetadata` is part of this re-export and stays in the Commerce contract by
// decision (recorded in `burncloud_commerce_contracts::pricing` and in #621). The move to Supply
// that S1-C anticipated is blocked on Supply having a real capability type; today the live
// capability definition is the one in the pricing document.
pub use burncloud_commerce_contracts::pricing::*;
