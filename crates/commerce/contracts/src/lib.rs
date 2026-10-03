//! Commerce-owned contracts for the BurnCloud platform.
//!
//! This package carries business semantics that belong to the Commerce domain and
//! must not be sourced from the Interfaces layer. It has no dependency on
//! `burncloud-common`, on service or database packages, or on any database/network
//! runtime, so domain packages can depend on Commerce contracts without creating a
//! package cycle.
//!
//! S1-A currently owns the nanodollar money conversion contract here. The legacy
//! `burncloud_common::price_u64` path re-exports this implementation for existing
//! consumers until the consumer switch is completed in later S1 subtasks.

pub mod price_u64;
