#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    reason = "golden contract test: unwrap/expect is the intended failure signal and the DTOs intentionally use plain types"
)]
//! Golden contract for the Traffic protocol and scheduling types (S1-E).
//!
//! Three promises cross a version boundary and are pinned here:
//!
//! * [`TrafficColor::as_char`] produces the single-character value stored in
//!   `router_logs.traffic_color`, and `Display` agrees with it. The JSON encoding is lowercase and
//!   the default is `Yellow`.
//! * [`OpenAIChatRequest`] keeps `#[serde(flatten)] extra`, so provider-specific fields survive a
//!   round trip instead of being dropped.
//! * `OpenAIChat*` field names and optionality are unchanged, because the HTTP entry point hands
//!   this DTO straight to the adapters.
//!
//! The tests import from `burncloud_traffic_contracts` only, so they keep working after the
//! `burncloud_common` compatibility layer is removed.

use burncloud_traffic_contracts::{
    OpenAIChatChoice, OpenAIChatMessage, OpenAIChatRequest, OpenAIChatResponse, TrafficColor,
};

#[test]
fn traffic_color_chars_are_the_storage_encoding() {
    // These three characters are written into router_logs.traffic_color.
    assert_eq!(TrafficColor::Green.as_char(), 'G');
    assert_eq!(TrafficColor::Yellow.as_char(), 'Y');
    assert_eq!(TrafficColor::Red.as_char(), 'R');

    // Display agrees with as_char, since the log column and the display path must not diverge.
    for color in [TrafficColor::Green, TrafficColor::Yellow, TrafficColor::Red] {
        assert_eq!(color.to_string(), color.as_char().to_string());
    }

    // Yellow is the default; a defaulted value must be the middle class, not the cheapest.
    assert_eq!(TrafficColor::default(), TrafficColor::Yellow);
}

#[test]
fn traffic_color_json_encoding_is_lowercase() {
    assert_eq!(
        serde_json::to_string(&TrafficColor::Green).unwrap(),
        "\"green\""
    );
    assert_eq!(
        serde_json::to_string(&TrafficColor::Yellow).unwrap(),
        "\"yellow\""
    );
    assert_eq!(
        serde_json::to_string(&TrafficColor::Red).unwrap(),
        "\"red\""
    );
    for color in [TrafficColor::Green, TrafficColor::Yellow, TrafficColor::Red] {
        let json = serde_json::to_string(&color).unwrap();
        assert_eq!(serde_json::from_str::<TrafficColor>(&json).unwrap(), color);
    }
    // The variants stay comparable and hashable: the shaper keys buckets by colour.
    use std::collections::HashSet;
    let set: HashSet<TrafficColor> = [TrafficColor::Green, TrafficColor::Green]
        .into_iter()
        .collect();
    assert_eq!(set.len(), 1);
    assert_ne!(TrafficColor::Green, TrafficColor::Red);
}

#[test]
fn chat_request_keeps_unknown_fields_through_the_round_trip() {
    // Provider-specific passthrough is the reason `extra` exists. Dropping the flatten attribute
    // would silently discard these fields.
    let raw = serde_json::json!({
        "model": "gpt-4o",
        "messages": [{"role": "user", "content": "hi"}],
        "temperature": 0.25,
        "max_tokens": 128,
        "stream": true,
        "top_p": 0.9,
        "response_format": {"type": "json_object"},
        "vendor_extension": [1, 2, 3]
    });

    let req: OpenAIChatRequest = serde_json::from_value(raw.clone()).unwrap();
    assert_eq!(req.model, "gpt-4o");
    assert_eq!(req.messages.len(), 1);
    assert_eq!(req.messages[0].role, "user");
    assert_eq!(req.messages[0].content, "hi");
    assert_eq!(req.temperature, Some(0.25));
    assert_eq!(req.max_tokens, Some(128));
    assert!(req.stream);

    // The unknown fields land in `extra` ...
    assert_eq!(req.extra.get("top_p"), Some(&serde_json::json!(0.9)));
    assert_eq!(
        req.extra.get("response_format"),
        Some(&serde_json::json!({"type": "json_object"}))
    );
    assert_eq!(
        req.extra.get("vendor_extension"),
        Some(&serde_json::json!([1, 2, 3]))
    );

    // ... and come back out unchanged.
    let back = serde_json::to_value(&req).unwrap();
    assert_eq!(back["top_p"], serde_json::json!(0.9));
    assert_eq!(
        back["response_format"],
        serde_json::json!({"type": "json_object"})
    );
    assert_eq!(back["vendor_extension"], serde_json::json!([1, 2, 3]));
    assert_eq!(back["model"], serde_json::json!("gpt-4o"));
}

#[test]
fn chat_request_defaults_match_the_current_behaviour() {
    // Only `model` and `messages` are required; the adapters rely on the defaults for the rest.
    let minimal = serde_json::json!({
        "model": "claude-3-5-sonnet",
        "messages": []
    });
    let req: OpenAIChatRequest = serde_json::from_value(minimal).unwrap();
    assert_eq!(req.temperature, None);
    assert_eq!(req.max_tokens, None);
    assert!(!req.stream, "stream defaults to false");
    assert!(req.extra.is_empty());

    // A request without `model` or `messages` is still an error.
    assert!(
        serde_json::from_value::<OpenAIChatRequest>(serde_json::json!({"model": "x"})).is_err()
    );
    assert!(
        serde_json::from_value::<OpenAIChatRequest>(serde_json::json!({"messages": []})).is_err()
    );
}

#[test]
fn chat_message_round_trips_the_message_level_fields_it_receives() {
    // `chat_request_keeps_unknown_fields_through_the_round_trip` proves that *request-level*
    // extras survive. This test covers the other path: an entry of `messages`. `OpenAIChatMessage`
    // now carries its own `#[serde(flatten)] extra` map, so the fields a client sends on a message
    // (`name`, `tool_calls`, `tool_call_id`, `refusal`, …) survive deserialisation and
    // re-serialisation instead of being dropped before the adapters can forward them.
    //
    // Before the struct was widened this file pinned the opposite (the loss); the assertions below
    // are that same case rewritten to pin preservation, which is the contract the adapters need.
    let raw = serde_json::json!({
        "model": "gpt-4o",
        "messages": [{
            "role": "assistant",
            "content": "calling a tool",
            "name": "planner",
            "tool_calls": [{"id": "call_1", "type": "function"}],
            "tool_call_id": "call_1",
            "refusal": null
        }]
    });

    let req: OpenAIChatRequest = serde_json::from_value(raw).unwrap();

    assert_eq!(req.messages.len(), 1);
    assert_eq!(req.messages[0].role, "assistant");
    assert_eq!(req.messages[0].content, "calling a tool");

    // The message-level fields land in the message's own flattened map.
    assert_eq!(
        req.messages[0].extra.get("name"),
        Some(&serde_json::json!("planner"))
    );
    assert_eq!(
        req.messages[0].extra.get("tool_call_id"),
        Some(&serde_json::json!("call_1"))
    );
    assert!(
        req.messages[0].extra.contains_key("tool_calls"),
        "a tool_calls payload must not be dropped on a message entry"
    );
    assert!(
        req.messages[0].extra.contains_key("refusal"),
        "an explicit null must be preserved, not silently dropped"
    );

    // And they survive the round trip: this is what the adapters forward.
    let back = serde_json::to_value(&req).unwrap();
    let message = &back["messages"][0];
    assert_eq!(message["role"], serde_json::json!("assistant"));
    assert_eq!(message["content"], serde_json::json!("calling a tool"));
    assert_eq!(message["name"], serde_json::json!("planner"));
    assert_eq!(message["tool_call_id"], serde_json::json!("call_1"));
    assert_eq!(
        message["tool_calls"],
        serde_json::json!([{"id": "call_1", "type": "function"}])
    );
    assert!(
        message["refusal"].is_null(),
        "the explicit null must round-trip as null, not disappear"
    );
    // And nothing leaked upward into `extra`: the two layers stay separate.
    assert!(
        req.extra.is_empty(),
        "message-level fields must not surface as request-level extras: {:?}",
        req.extra
    );
}

#[test]
fn a_message_field_of_the_wrong_type_is_rejected_only_for_the_declared_fields() {
    // The counterpart to the known gap above: because only `role` and `content` are typed, a
    // wrongly-typed *declared* field is still rejected ...
    for (json, why) in [
        (
            serde_json::json!({"model": "gpt-4o", "messages": [{"role": 1, "content": "hi"}]}),
            "`role` given as a number",
        ),
        (
            serde_json::json!({"model": "gpt-4o", "messages": [{"role": "user", "content": 7}]}),
            "`content` given as a number",
        ),
    ] {
        let result = serde_json::from_value::<OpenAIChatRequest>(json);
        assert!(result.is_err(), "{why} must be rejected, but it parsed");
    }

    // ... while an undeclared field is not validated at all: a value of the wrong type is discarded
    // rather than rejected. Recorded so the silence is not mistaken for strict validation.
    let tolerated = serde_json::json!({
        "model": "gpt-4o",
        "messages": [{"role": "user", "content": "hi", "tool_calls": "not-a-list"}]
    });
    let req: OpenAIChatRequest = serde_json::from_value(tolerated)
        .expect("an undeclared message field is ignored, so this shape must parse");
    assert_eq!(req.messages[0].content, "hi");
}

#[test]
fn chat_response_shape_is_unchanged() {
    let response = OpenAIChatResponse {
        id: "chatcmpl-1".to_string(),
        object: "chat.completion".to_string(),
        created: 1_700_000_000,
        model: "gpt-4o".to_string(),
        choices: vec![OpenAIChatChoice {
            index: 0,
            message: OpenAIChatMessage {
                role: "assistant".to_string(),
                content: "hello".to_string(),
                extra: Default::default(),
            },
            finish_reason: Some("stop".to_string()),
        }],
    };

    let value = serde_json::to_value(&response).unwrap();
    assert_eq!(value["id"], serde_json::json!("chatcmpl-1"));
    assert_eq!(value["object"], serde_json::json!("chat.completion"));
    assert_eq!(value["created"], serde_json::json!(1_700_000_000u64));
    assert_eq!(value["choices"][0]["index"], serde_json::json!(0));
    assert_eq!(
        value["choices"][0]["message"]["role"],
        serde_json::json!("assistant")
    );
    assert_eq!(
        value["choices"][0]["finish_reason"],
        serde_json::json!("stop")
    );

    let back: OpenAIChatResponse = serde_json::from_value(value).unwrap();
    assert_eq!(back.choices.len(), 1);
    assert_eq!(back.choices[0].message.content, "hello");

    // `finish_reason` is optional and may be absent entirely.
    let no_reason: OpenAIChatChoice = serde_json::from_value(serde_json::json!({
        "index": 0,
        "message": {"role": "assistant", "content": "x"}
    }))
    .unwrap();
    assert_eq!(no_reason.finish_reason, None);
}
