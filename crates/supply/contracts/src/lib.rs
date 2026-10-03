//! Supply-owned contracts for the BurnCloud platform.
//!
//! This package carries the supply facts that used to live in the Interfaces layer: what a channel
//! (provider) is, what abilities it advertises, and the channel-type encoding. It is a leaf package
//! -- no `burncloud-*` dependency, no database framework, no runtime -- so any domain can depend on
//! the supply contract without creating a package cycle or inheriting infrastructure.
//!
//! S1-C moved [`ChannelType`], [`Channel`] and [`Ability`] here. The legacy
//! `burncloud_common::types::*` paths re-export them unchanged.

pub mod types;

pub use types::*;
