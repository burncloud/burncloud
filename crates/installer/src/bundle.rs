//! Bundle module for offline installation support
//!
//! # ⚠️ STUB IMPLEMENTATION
//! This module is currently a stub. The actual bundle creation and verification
//! logic is not yet implemented.

use serde::{Deserialize, Serialize};

/// Bundle manifest information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub version: String,
    pub created_at: String,
    pub software: Vec<String>,
}

/// Bundle creator (stub)
///
/// # ⚠️ STUB: Bundle creation is NOT implemented yet.
pub struct BundleCreator;

impl BundleCreator {
    pub fn new() -> Self {
        BundleCreator
    }
}

impl Default for BundleCreator {
    fn default() -> Self {
        Self::new()
    }
}

/// Bundle verifier (stub)
///
/// # ⚠️ STUB: Bundle verification is NOT implemented yet.
pub struct BundleVerifier;

impl BundleVerifier {
    pub fn new() -> Self {
        BundleVerifier
    }
}

impl Default for BundleVerifier {
    fn default() -> Self {
        Self::new()
    }
}
