#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    reason = "golden contract test: unwrap/expect is the intended failure signal and the DTOs intentionally use plain types"
)]
//! Malformed-input contract for the Traffic protocol types (#633, "非法输入的反序列化错误").
//!
//! `traffic_contract.rs` pins what these DTOs *accept* and how they re-encode it. This file pins
//! what they must **refuse**. The distinction matters because two of the types are deliberately
//! permissive in a way that is easy to widen by accident:
//!
//! * [`OpenAIChatRequest`] carries `#[serde(flatten)] extra`, so *unknown* fields must survive.
//!   That makes it tempting to assume anything unrecognised is tolerated — but a request with no
//!   `model`, or with a `messages` entry that is not an object, is still unusable and must fail
//!   loudly rather than deserialise into a request that silently sends the wrong thing upstream.
//! * [`TrafficColor`] is written into `router_logs.traffic_color` as a single character, so a
//!   value that is not one of the three known classes must be rejected. Defaulting an unknown
//!   colour would relabel real traffic in the log, which no caller could detect afterwards.
//!
//! Every assertion checks the *failure*, and each is written so that a permissive change makes it
//! fail: if a required field becomes optional, or an unknown enum variant starts to fall back to a
//! default, these tests break instead of quietly passing.

use burncloud_traffic_contracts::{OpenAIChatRequest, TrafficColor};

/// Assert `json` fails to deserialise, and report the input when it unexpectedly succeeds.
fn assert_rejected<T: serde::de::DeserializeOwned + std::fmt::Debug>(json: &str, why: &str) {
    if let Ok(value) = serde_json::from_str::<T>(json) {
        panic!("{why}: expected rejection, but parsed as {value:?} from {json}");
    }
}

#[test]
fn a_request_missing_a_required_field_is_rejected() {
    // `model` and `messages` are both required. Without `model` there is nothing to route to, and
    // the adapters would send an upstream call that cannot be satisfied.
    assert_rejected::<OpenAIChatRequest>(
        r#"{"messages":[{"role":"user","content":"hi"}]}"#,
        "a request without `model`",
    );

    // Without `messages` there is no conversation to adapt.
    assert_rejected::<OpenAIChatRequest>(r#"{"model":"gpt-4o"}"#, "a request without `messages`");

    // An empty object is the weakest form of both.
    assert_rejected::<OpenAIChatRequest>(r#"{}"#, "an empty request body");
}

#[test]
fn a_message_that_is_not_an_object_is_rejected() {
    // A bare string in the `messages` array is a common shape in other protocols; accepting it
    // here would mean inventing a role, so it must fail.
    assert_rejected::<OpenAIChatRequest>(
        r#"{"model":"gpt-4o","messages":["hello"]}"#,
        "a message given as a plain string",
    );

    // `role` and `content` are both required on a message.
    assert_rejected::<OpenAIChatRequest>(
        r#"{"model":"gpt-4o","messages":[{"content":"hi"}]}"#,
        "a message without `role`",
    );
    assert_rejected::<OpenAIChatRequest>(
        r#"{"model":"gpt-4o","messages":[{"role":"user"}]}"#,
        "a message without `content`",
    );
}

#[test]
fn a_wrongly_typed_field_is_rejected_rather_than_coerced() {
    // `messages` must be a sequence, not a single object.
    assert_rejected::<OpenAIChatRequest>(
        r#"{"model":"gpt-4o","messages":{"role":"user","content":"hi"}}"#,
        "`messages` given as an object",
    );

    // `model` must be a string; a number is not a model name.
    assert_rejected::<OpenAIChatRequest>(
        r#"{"model":42,"messages":[]}"#,
        "`model` given as a number",
    );

    // `temperature` is `Option<f32>`, so a non-numeric value must not be silently dropped.
    assert_rejected::<OpenAIChatRequest>(
        r#"{"model":"gpt-4o","messages":[],"temperature":"hot"}"#,
        "`temperature` given as a string",
    );

    // `stream` is `bool` with a default; a string must not coerce to a truthy default.
    assert_rejected::<OpenAIChatRequest>(
        r#"{"model":"gpt-4o","messages":[],"stream":"yes"}"#,
        "`stream` given as a string",
    );
}

#[test]
fn an_out_of_range_numeric_field_is_rejected() {
    // `max_tokens` is `u32`. A negative count is meaningless and must not wrap.
    assert_rejected::<OpenAIChatRequest>(
        r#"{"model":"gpt-4o","messages":[],"max_tokens":-1}"#,
        "`max_tokens` given as a negative number",
    );

    // One past `u32::MAX` is the boundary on the other side.
    assert_rejected::<OpenAIChatRequest>(
        r#"{"model":"gpt-4o","messages":[],"max_tokens":4294967296}"#,
        "`max_tokens` one past the u32 maximum",
    );
}

#[test]
fn an_unknown_traffic_colour_is_rejected_instead_of_defaulting() {
    // The three known classes are lowercase strings. Anything else must fail: falling back to the
    // default would write a different colour into router_logs.traffic_color than the caller asked
    // for, and the log is the only record of the scheduling decision.
    for unknown in [
        r#""blue""#,
        r#""GREEN""#,
        r#""""#,
        r#""green ""#,
        r#"1"#,
        r#"null"#,
    ] {
        assert_rejected::<TrafficColor>(unknown, "an unknown traffic colour");
    }
}

#[test]
fn the_three_known_traffic_colours_still_parse() {
    // The counterpart to the test above: rejection must come from *unknown* values, not from a
    // parser that rejects everything. Without this, a broken `TrafficColor` deserialiser would
    // make the malformed-input test pass for the wrong reason.
    assert_eq!(
        serde_json::from_str::<TrafficColor>(r#""green""#).unwrap(),
        TrafficColor::Green
    );
    assert_eq!(
        serde_json::from_str::<TrafficColor>(r#""yellow""#).unwrap(),
        TrafficColor::Yellow
    );
    assert_eq!(
        serde_json::from_str::<TrafficColor>(r#""red""#).unwrap(),
        TrafficColor::Red
    );
}
