//! Commerce-owned contracts for the BurnCloud platform.
//!
//! This package carries business semantics that belong to the Commerce domain and
//! must not be sourced from the Interfaces layer. It has no dependency on
//! `burncloud-common`, on service or database packages, or on any database/network
//! runtime, so domain packages can depend on Commerce contracts without creating a
//! package cycle.
//!
//! S1-A owns the nanodollar money conversion contract here; S1-B adds the price
//! and pricing-document value objects. The legacy `burncloud_common::price_u64`,
//! `burncloud_common::pricing_config` and `burncloud_common::types` paths re-export
//! these items for consumers that have not switched yet.

pub mod price_u64;
pub mod pricing;
