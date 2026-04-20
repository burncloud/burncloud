//! Database operations for model_ domain (model capability metadata).
//!
//! This crate is the canonical home of the `model_capabilities` table
//! (HuggingFace-style metadata). Other entities that used to live under the
//! legacy `database-models` crate have been moved to their proper domain
//! crates:
//!   - `user_api_key.rs` → `database-user`
//!   - `channel_ability.rs`, `channel_protocol_config.rs`, `channel_provider.rs` → `database-channel`
//!   - `billing_price.rs`, `billing_tiered_price.rs` → `database-billing`
//!   - `router_video_task.rs` → `database-router`

mod common;
mod model_capability;

pub use common::current_timestamp;
pub use model_capability::{ModelDatabase, ModelInfo};

/// Spec-aligned alias: `model_capabilities` row type.
/// The actual metadata is loaded into `ModelInfo` — this alias matches the
/// naming scheme while the richer metadata struct survives as the canonical type.
pub type ModelCapability = ModelInfo;

/// Spec-aligned alias: operation type for the `model_capabilities` table.
/// Currently maps to `ModelDatabase`, the crate-level controller that holds
/// the CRUD stubs for this table.
pub type ModelCapabilityModel = ModelDatabase;

pub use burncloud_database::DatabaseError;
