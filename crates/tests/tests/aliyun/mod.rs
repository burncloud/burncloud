#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
//! Aliyun ECS integration tests
//!
//! These tests verify BurnCloud bundle installation on Aliyun Windows ECS
//!
//! Test files are organized by functionality for easier debugging:
//! - step1_instance.rs - Create/Delete ECS instances
//! - step2_ssh.rs - SSH installation and connection
//! - step3_upload.rs - File upload via SFTP
//! - step4_install.rs - Bundle installation
//! - step5_verify.rs - Installation verification
//! - e2e_test.rs - Full end-to-end test

mod aliyun_api;
mod e2e_test;

// Step-by-step test modules
mod step1_instance;
mod step2_ssh;
mod step3_upload;
mod step4_install;
mod step5_verify;

pub use aliyun_api::AliyunECS;
pub use e2e_test::BundleE2ETest;
