#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types)]
//! Rejection behaviour of the Supply contract (#633).
//!
//! `channel_contract.rs` covers what the contract accepts: the discriminant table, the `type` field
//! name, absent optional columns, the aliases. What was missing is what the contract **rejects**.
//! That matters because rows come from operator input and from a sync path, and a field of the wrong
//! type is exactly where a lenient decoder would store something plausible instead of failing: a
//! numeric `key` becoming `"123"`, a numeric `model` becoming `"0"`, a string `weight` becoming `1`.
//!
//! Three things this file learned the hard way and now encodes:
//!
//! 1. **A control case is mandatory.** The first draft's "minimal" channel was missing three of the
//!    eleven required fields, so every rejection assertion was passing for the wrong reason. The
//!    control test below parses the same fixture used by all the negative cases.
//! 2. **serde errors do not carry the field path.** `invalid type: string "1", expected i32` names
//!    no field, so asserting "the message contains the field name" cannot work. Each case therefore
//!    varies exactly one field, which is what makes the failure attributable.
//! 3. **Only real behaviour is asserted.** `type_` is an `i32`, not the enum, so an unknown wire
//!    value must *not* fail -- the opposite policy to everything else here, and a plausible
//!    "tidy-up" that would break it.

use burncloud_supply_contracts::{Ability, Channel, ChannelType};
use serde_json::json;

/// Every required field of `Channel`, correctly typed. Optional fields are omitted on purpose, since
/// absence is already covered by `channel_contract.rs`.
fn channel_json() -> serde_json::Value {
    json!({
        "id": 1,
        "type": 1,
        "key": "sk-x",
        "status": 1,
        "name": "chan",
        "weight": 0,
        "models": "gpt-4o",
        "group": "default",
        "used_quota": 0,
        "priority": 0,
        "auto_ban": 0
    })
}

fn ability_json() -> serde_json::Value {
    json!({
        "group": "default",
        "model": "gpt-4o",
        "channel_id": 1,
        "enabled": 1,
        "priority": 0,
        "weight": 0
    })
}

/// Replace one field, then require the parse to fail. `what` appears in the panic message so a
/// failing case is identifiable without a debugger.
fn assert_rejected(mut value: serde_json::Value, field: &str, what: &str) {
    value[field] = what_value(what);
    match serde_json::from_value::<Channel>(value) {
        Ok(channel) => panic!(
            "{field} as {what} was accepted, producing {field}={:?} -- the contract must reject it",
            channel.name
        ),
        Err(e) => {
            // The error text is kept in the message: serde does not report the field path, so the
            // reported type is the evidence that the rejection came from this field.
            assert!(
                !e.to_string().is_empty(),
                "rejection of {field} as {what} produced an empty error"
            );
        }
    }
}

fn what_value(what: &str) -> serde_json::Value {
    match what {
        "a string" => json!("1"),
        "a float" => json!(1.5),
        "a number" => json!(12345),
        "a boolean" => json!(true),
        "an object" => json!({"a": 1}),
        "an array" => json!([1, 2]),
        other => panic!("unknown variant: {other}"),
    }
}

#[test]
fn the_fixture_parses_so_every_rejection_below_is_about_types() {
    // If this fails, the negative tests below prove nothing: they would be rejecting an
    // already-invalid object rather than the single field they vary.
    let channel: Channel =
        serde_json::from_value(channel_json()).expect("the fixture must be a valid Channel");
    assert_eq!(channel.id, 1);
    assert_eq!(channel.type_, ChannelType::OpenAI as i32);
    assert_eq!(channel.key, "sk-x");
    assert_eq!(channel.models, "gpt-4o");
    assert_eq!(channel.group, "default");
    assert_eq!(channel.used_quota, 0);
    assert_eq!(channel.auto_ban, 0);

    let ability: Ability =
        serde_json::from_value(ability_json()).expect("the fixture must be a valid Ability");
    assert_eq!(ability.channel_id, 1);
    assert_eq!(ability.enabled, 1);
}

#[test]
fn string_fields_reject_non_strings() {
    // `key` is credential material and `name`/`models`/`group` identify the channel and pick its
    // models. A decoder turning 12345 into "12345" would persist a wrong credential or model list.
    for field in ["key", "name", "models", "group"] {
        for what in ["a number", "a boolean", "an object", "an array"] {
            assert_rejected(channel_json(), field, what);
        }
    }
}

#[test]
fn integer_fields_reject_strings_floats_and_booleans() {
    // These select the channel, its state and its share of traffic. serde_json does not coerce "1"
    // to 1, a fractional value has no integer representation, and `true` is not 1.
    for field in [
        "id",
        "type",
        "status",
        "weight",
        "used_quota",
        "priority",
        "auto_ban",
    ] {
        for what in ["a string", "a float", "a boolean"] {
            assert_rejected(channel_json(), field, what);
        }
    }
}

#[test]
fn a_missing_required_field_is_rejected() {
    // One required field removed at a time. This is the shape a partial row from an older schema
    // would have, and silently defaulting it would hide a broken migration.
    let required = [
        "id",
        "type",
        "key",
        "status",
        "name",
        "weight",
        "models",
        "group",
        "used_quota",
        "priority",
        "auto_ban",
    ];
    for field in required {
        let mut value = channel_json();
        value.as_object_mut().expect("object").remove(field);
        let result = serde_json::from_value::<Channel>(value);
        assert!(
            result.is_err(),
            "a channel without {field} was accepted; {field} must be required"
        );
    }
}

#[test]
fn optional_fields_reject_a_wrong_type_instead_of_falling_back_to_none() {
    // `Option<T>` accepts null and absence. It must not accept a value of the wrong type: the
    // fallback would silently drop a configured endpoint or a capacity cap.
    let wrong: [(&str, serde_json::Value); 4] = [
        ("base_url", json!(42)),
        ("rpm_cap", json!("600")), // a quota cap arriving as a string
        ("reservation_green", json!("0.5")), // a capacity share arriving as a string
        ("model_mapping", json!(7)), // a mapping arriving as a number
    ];
    for (field, bad) in wrong {
        let mut value = channel_json();
        value[field] = bad.clone();
        let result = serde_json::from_value::<Channel>(value);
        assert!(
            result.is_err(),
            "an optional field with the wrong type must be rejected, but {field}={bad} was accepted"
        );
    }

    // null and absence stay acceptable, so the strictness above did not break the nullable contract.
    let mut value = channel_json();
    value["base_url"] = json!(null);
    value["rpm_cap"] = json!(null);
    assert!(
        serde_json::from_value::<Channel>(value).is_ok(),
        "null optional fields must still be accepted"
    );
}

#[test]
fn abilities_are_validated_like_channels() {
    // A numeric model name would make an ability match the wrong model; a numeric group would put it
    // in the wrong group; a boolean `enabled` is not the i32 the column stores.
    for (field, bad) in [
        ("model", json!(123)),
        ("group", json!(0)),
        ("enabled", json!(true)),
        ("channel_id", json!("1")),
        ("priority", json!(1.5)),
        ("weight", json!("3")),
    ] {
        let mut value = ability_json();
        value[field] = bad.clone();
        let result = serde_json::from_value::<Ability>(value);
        assert!(
            result.is_err(),
            "ability field {field}={bad} must be rejected, but it was accepted"
        );
    }

    // A missing required field is rejected rather than defaulted.
    for field in [
        "group",
        "model",
        "channel_id",
        "enabled",
        "priority",
        "weight",
    ] {
        let mut value = ability_json();
        value.as_object_mut().expect("object").remove(field);
        assert!(
            serde_json::from_value::<Ability>(value).is_err(),
            "an ability without {field} was accepted; {field} must be required"
        );
    }
}

#[test]
fn an_unknown_channel_type_still_decodes_because_the_field_is_a_raw_integer() {
    // The opposite policy to everything above, and deliberate: keeping `type_` an i32 lets a value
    // from a newer New API release survive instead of failing the whole row, so the entry point can
    // decide what to do about it. A tidy-up that switched the field to `ChannelType` would silently
    // start rejecting those rows.
    for wire in [9999_i32, -7, 0, i32::MAX] {
        let mut value = channel_json();
        value["type"] = json!(wire);
        let channel = serde_json::from_value::<Channel>(value)
            .unwrap_or_else(|e| panic!("wire value {wire} must not fail the whole row: {e}"));
        assert_eq!(
            channel.type_, wire,
            "the raw value must be preserved verbatim"
        );
    }

    // The enum keeps its own mapping separately.
    assert_eq!(ChannelType::from(9999), ChannelType::Unknown);
    assert_eq!(ChannelType::from(-7), ChannelType::Unknown);
    assert_eq!(ChannelType::from(0), ChannelType::Unknown);
    assert_eq!(ChannelType::from(1), ChannelType::OpenAI);
}
