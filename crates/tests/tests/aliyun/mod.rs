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
