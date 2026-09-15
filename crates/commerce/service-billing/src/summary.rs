//! Billing summary aggregation — re-exports and delegates to the existing
//! `BillingService` in `service-router-log`.
//!
//! This module exists so that `service-billing` is the single import point for
//! all billing-related types and functions, keeping the dependency graph clean
//! (server → service-billing instead of server → service-router-log for billing).

// Re-export the domain types so callers don't need to depend on
// service-router-log or database-router directly.
pub use burncloud_database_router::{BillingModelSummary, BillingSummary};
pub use burncloud_service_router_log::BillingService;
