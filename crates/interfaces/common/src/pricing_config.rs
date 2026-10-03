// Compatibility shim: burncloud_common::pricing_config now re-exports the Commerce-owned
// pricing document contract. The schema implementation (and its tests) moved to
// `burncloud_commerce_contracts::pricing` in S1-B; nothing is defined here any more.
//
// This module keeps `burncloud_common::pricing_config::{PricingConfig, ...}` and the
// root-level `burncloud_common::{PricingConfig, ...}` glob export working for existing
// consumers until they switch to the Commerce contract.
//
// S1 note: `ModelMetadata` is still part of this re-export. It is model-capability truth
// owned by Supply and will move in S1-C; keeping it here avoids changing its JSON shape twice.
pub use burncloud_commerce_contracts::pricing::*;
