//! Supply-owned persistence for model capability truth.
//!
//! The historical `model_capabilities` table mixes capability columns with two USD price columns.
//! #621 resolves the code ownership without rewriting historical migrations:
//!
//! - capability persistence is owned here by `ModelCapabilityModel`;
//! - Commerce remains the canonical owner of pricing and of the pricing-document DTO;
//! - Traffic may project a parsed pricing document into `ModelCapabilityInput`, but it does not
//!   execute SQL against this table;
//! - `input_price` / `output_price` are legacy compatibility projections, not canonical price
//!   truth.
//!
//! The former HuggingFace-shaped `ModelInfo` and its no-op CRUD controller were removed because
//! no runtime table matched that shape and every operation was a stub.

mod common;
mod model_capability;

pub use common::current_timestamp;
pub use model_capability::{ModelCapability, ModelCapabilityInput, ModelCapabilityModel};

pub use burncloud_database::DatabaseError;
