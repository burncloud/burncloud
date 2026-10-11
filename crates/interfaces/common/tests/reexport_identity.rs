#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "compatibility test: fail-fast expectations are the intended signal and the fixtures are known-good constants"
)]
//! Re-export identity and JSON compatibility for `burncloud_common` (#633).
//!
//! `surface_inventory.rs` guards the crate's *declarations* and records two limits in its own
//! header: it does not enumerate the members of the re-export blocks, and it does not cover the
//! nanodollar helpers that `common::*` still forwards. Those two surfaces are exactly what
//! existing consumers compile against, so this file closes them:
//!
//! * **identity** — a compatibility path must resolve to the *owner's* type, not to a lookalike.
//!   Each check binds the value to a variable annotated with the owner's type: Rust has no
//!   implicit conversion between distinct types, so a second declaration would fail to compile.
//! * **JSON compatibility** — a compatibility path must serialise to the bytes the owner
//!   produces, and the expected encoding is written out explicitly rather than inferred from the
//!   owner, so the encoding itself is pinned.

use burncloud_common::types;

/// Every compatibility path in `common::types` must resolve to the owner's type.
#[test]
fn compatibility_paths_resolve_to_the_owner_types() {
    let channel_type: burncloud_supply_contracts::ChannelType = types::ChannelType::OpenAI;
    assert_eq!(channel_type as i32, 1);

    let color: burncloud_traffic_contracts::TrafficColor = types::TrafficColor::Green;
    assert_eq!(color.as_char(), 'G');

    let currency: burncloud_commerce_contracts::pricing::Currency = types::Currency::USD;
    assert_eq!(currency.code(), "USD");

    let message: burncloud_traffic_contracts::OpenAIChatMessage = types::OpenAIChatMessage {
        role: "user".to_string(),
        content: "hi".to_string(),
        extra: Default::default(),
    };
    assert_eq!(message.role, "user");
}

/// The legacy alias names are the owner's items, not wrappers around them.
#[test]
fn legacy_aliases_are_the_owner_items() {
    // A local wrapper would have the same signature, so the address is what proves the identity.
    let legacy_nano: fn(f64) -> i64 = types::dollars_to_nanodollars;
    let owner_nano: fn(f64) -> i64 = burncloud_commerce_contracts::price_u64::dollars_to_nano;
    assert_eq!(legacy_nano as usize, owner_nano as usize);

    let legacy_dollars: fn(i64) -> f64 = types::nanodollars_to_dollars;
    let owner_dollars: fn(i64) -> f64 = burncloud_commerce_contracts::price_u64::nano_to_dollars;
    assert_eq!(legacy_dollars as usize, owner_dollars as usize);

    assert_eq!(
        types::NANODOLLAR_SCALE,
        burncloud_commerce_contracts::price_u64::NANO_PER_DOLLAR
    );
    assert_eq!(burncloud_common::NANODOLLAR_SCALE, types::NANODOLLAR_SCALE);
    assert_eq!(burncloud_common::NANO_PER_DOLLAR, types::NANODOLLAR_SCALE);
}

/// The compatibility paths must serialise to the owner's bytes and to the pinned encoding.
#[test]
fn compatibility_paths_serialise_like_the_owners() {
    assert_eq!(
        serde_json::to_string(&types::ChannelType::OpenAI).unwrap(),
        "\"OpenAI\""
    );
    assert_eq!(
        serde_json::to_string(&types::ChannelType::OpenAI).unwrap(),
        serde_json::to_string(&burncloud_supply_contracts::ChannelType::OpenAI).unwrap()
    );

    // The three traffic colours are a storage promise: one lowercase word, not the variant case.
    assert_eq!(
        serde_json::to_string(&types::TrafficColor::Green).unwrap(),
        "\"green\""
    );
    assert_eq!(
        serde_json::to_string(&types::TrafficColor::Red).unwrap(),
        serde_json::to_string(&burncloud_traffic_contracts::TrafficColor::Red).unwrap()
    );

    // Stored pricing documents keep the lowercase currency code.
    assert_eq!(
        serde_json::to_string(&types::Currency::CNY).unwrap(),
        "\"cny\""
    );

    // Message-level fields are fixed by the contract: role and content, in that order.
    let message = types::OpenAIChatMessage {
        role: "user".to_string(),
        content: "hi".to_string(),
        extra: Default::default(),
    };
    assert_eq!(
        serde_json::to_string(&message).unwrap(),
        r#"{"role":"user","content":"hi"}"#
    );
}
