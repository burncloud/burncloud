//! Billing and Quota Test Suite
//!
//! This module contains comprehensive tests for the billing and quota system,
//! covering all P0 and P1 requirements from Issue #57.
//!
//! Test Categories:
//! - BL-01: Token counting (token_count.rs)
//! - BL-02: Amount calculation with i64 nanodollar precision (price_calc.rs)
//! - BL-03: Tiered pricing calculation (price_calc.rs)
//! - BL-04: User balance deduction (balance_ops.rs)
//! - BL-05: Recharge functionality (balance_ops.rs)
//! - BL-06: Quota checking (quota_check.rs)
//!
//! Key Requirements (from CLAUDE.md):
//! - All amounts use i64 nanodollars (9 decimal precision: $1 = 1_000_000_000)
//! - NO floating point arithmetic for financial calculations
//! - NO rust_decimal - use native i64
//! - Concurrency-safe balance operations using CAS pattern
//! - Dual database compatibility (SQLite + PostgreSQL)

mod token_count;
mod price_calc;
mod balance_ops;
mod quota_check;
