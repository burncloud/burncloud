//! Database channel crate for BurnCloud
//!
//! This crate provides database model implementations for channel and ability management,
//! extracted from `database-models` to align with the Database ↔ Service matrix (spec §1.6).

mod channel;
mod common;

pub use channel::ChannelModel;

pub use burncloud_database::DatabaseError;
