//! Logging Test Suite
//!
//! This module contains comprehensive tests for the logging system,
//! covering all P1 requirements from Issue #59.
//!
//! Test Categories:
//! - LG-01: Request logging (log_types.rs)
//! - LG-02: Log query API (api/log.rs)
//! - LG-03: User usage statistics (usage_stats.rs)
//!
//! Key Requirements (from CLAUDE.md):
//! - All costs use i64 nanodollars (9 decimal precision: $1 = 1_000_000_000)
//! - NO floating point arithmetic for financial calculations
//! - Token counts stored as i32 (prompt) and i32 (completion)
//! - Latency stored as i64 milliseconds

mod log_types;
mod usage_stats;
