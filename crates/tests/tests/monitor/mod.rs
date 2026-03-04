//! Monitor Test Suite
//!
//! This module contains comprehensive tests for the system monitoring,
//! covering P2 requirements from Issue #59.
//!
//! Test Categories:
//! - MN-01: System monitoring API (types.rs)
//!
//! Key Requirements (from CLAUDE.md):
//! - CPU usage as f32 percentage (0-100)
//! - Memory in bytes (u64)
//! - Disk info with mount points
//! - System metrics with timestamp

mod types;
