#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    reason = "golden contract test: unwrap/expect is the intended failure signal and the DTOs intentionally use plain types"
)]
//! Golden contract for the Supply channel/ability types (S1-C).
//!
//! Two promises are pinned here because they cross version boundaries:
//!
//! * [`ChannelType`] discriminants are the New API wire encoding, and `From<i32>` has no error case:
//!   any unlisted value becomes `Unknown`. Both directions (`as i32`, `From<i32>`) are asserted over
//!   the whole listed range plus the gaps (28, 29, 30, 32) and out-of-range values.
//! * [`Channel`] serializes its `type_` field as `"type"`, which is the historical JSON name.
//!
//! The tests deliberately import from `burncloud_supply_contracts` only, so they keep working after
//! the `burncloud_common` compatibility layer is removed.

use burncloud_supply_contracts::{Ability, Channel, ChannelAbility, ChannelProvider, ChannelType};

/// Every listed discriminant, in ascending order.
///
/// 55 of the 56 variants carry an explicit value; `Dummy` is the only one without a value, so it is
/// covered separately in the fallback test. The numbering skips 28, 29, 30 and 32 and ends at 58,
/// which is why neither the variant count nor the maximum value is a round number.
const LISTED: &[(i32, ChannelType)] = &[
    (0, ChannelType::Unknown),
    (1, ChannelType::OpenAI),
    (2, ChannelType::Midjourney),
    (3, ChannelType::Azure),
    (4, ChannelType::Ollama),
    (5, ChannelType::MidjourneyPlus),
    (6, ChannelType::OpenAIMax),
    (7, ChannelType::OhMyGPT),
    (8, ChannelType::Custom),
    (9, ChannelType::AILS),
    (10, ChannelType::AIProxy),
    (11, ChannelType::PaLM),
    (12, ChannelType::API2GPT),
    (13, ChannelType::AIGC2D),
    (14, ChannelType::Anthropic),
    (15, ChannelType::Baidu),
    (16, ChannelType::Zhipu),
    (17, ChannelType::Ali),
    (18, ChannelType::Xunfei),
    (19, ChannelType::Qihoo360),
    (20, ChannelType::OpenRouter),
    (21, ChannelType::AIProxyLibrary),
    (22, ChannelType::FastGPT),
    (23, ChannelType::Tencent),
    (24, ChannelType::Gemini),
    (25, ChannelType::Moonshot),
    (26, ChannelType::ZhipuV4),
    (27, ChannelType::Perplexity),
    (31, ChannelType::LingYiWanWu),
    (33, ChannelType::Aws),
    (34, ChannelType::Cohere),
    (35, ChannelType::MiniMax),
    (36, ChannelType::SunoAPI),
    (37, ChannelType::Dify),
    (38, ChannelType::Jina),
    (39, ChannelType::Cloudflare),
    (40, ChannelType::SiliconFlow),
    (41, ChannelType::VertexAi),
    (42, ChannelType::Mistral),
    (43, ChannelType::DeepSeek),
    (44, ChannelType::MokaAI),
    (45, ChannelType::VolcEngine),
    (46, ChannelType::BaiduV2),
    (47, ChannelType::Xinference),
    (48, ChannelType::Xai),
    (49, ChannelType::Coze),
    (50, ChannelType::Kling),
    (51, ChannelType::Jimeng),
    (52, ChannelType::Vidu),
    (53, ChannelType::Submodel),
    (54, ChannelType::DoubaoVideo),
    (55, ChannelType::Sora),
    (56, ChannelType::Replicate),
    (57, ChannelType::Zai),
    (58, ChannelType::NewApi),
];

fn full_channel() -> Channel {
    Channel {
        id: 42,
        type_: ChannelType::Anthropic as i32,
        key: "sk-secret".to_string(),
        status: 1,
        name: "claude-prod".to_string(),
        weight: 10,
        created_time: Some(1_700_000_000),
        test_time: Some(1_700_000_100),
        response_time: Some(120),
        base_url: Some("https://api.anthropic.com/v1".to_string()),
        models: "claude-3-5-sonnet,claude-3-haiku".to_string(),
        group: "default".to_string(),
        used_quota: 1_234_567_890,
        model_mapping: Some(r#"{"claude-3":"claude-3-5-sonnet"}"#.to_string()),
        priority: 7,
        auto_ban: 1,
        other_info: Some("info".to_string()),
        tag: Some("prod".to_string()),
        setting: Some(r#"{"a":1}"#.to_string()),
        param_override: Some(r#"{"temperature":0.2}"#.to_string()),
        header_override: Some(r#"{"x-extra":"1"}"#.to_string()),
        remark: Some("primary channel".to_string()),
        api_version: Some("2024-02-01".to_string()),
        pricing_region: Some("international".to_string()),
        rpm_cap: Some(600),
        tpm_cap: Some(200_000),
        reservation_green: Some(0.5),
        reservation_yellow: Some(0.3),
        reservation_red: Some(0.2),
    }
}

#[test]
fn channel_type_discriminants_are_the_wire_encoding() {
    for (value, variant) in LISTED {
        assert_eq!(
            *variant as i32, *value,
            "discriminant mismatch for {variant:?}"
        );
        assert_eq!(
            ChannelType::from(*value),
            *variant,
            "From<i32> mismatch for {value}"
        );
    }
}

#[test]
fn unlisted_and_out_of_range_values_fall_back_to_unknown() {
    // Gaps in the New API numbering.
    for value in [28, 29, 30, 32] {
        assert_eq!(
            ChannelType::from(value),
            ChannelType::Unknown,
            "gap {value}"
        );
    }
    // Out of range, including negatives: there is no error case.
    for value in [-1, 59, 1000, i32::MAX, i32::MIN] {
        assert_eq!(
            ChannelType::from(value),
            ChannelType::Unknown,
            "out of range {value}"
        );
    }
    // `Dummy` is a Rust-only variant without an assigned wire value, so it takes the implicit
    // discriminant after `NewApi = 58`. It must not be produced by `From<i32>`: 59 maps to Unknown.
    assert_eq!(ChannelType::Dummy as i32, 59);
    assert_eq!(ChannelType::from(59), ChannelType::Unknown);
    assert_ne!(ChannelType::from(59), ChannelType::Dummy);
    // Its serde name is still the variant name, which is a compatibility surface of its own.
    assert_eq!(
        serde_json::to_string(&ChannelType::Dummy).unwrap(),
        "\"Dummy\""
    );
}

#[test]
fn channel_type_json_is_the_variant_name() {
    // serde uses the variant name; this is the encoding stored in configuration and logs.
    assert_eq!(
        serde_json::to_string(&ChannelType::OpenAI).unwrap(),
        "\"OpenAI\""
    );
    assert_eq!(
        serde_json::to_string(&ChannelType::VertexAi).unwrap(),
        "\"VertexAi\""
    );
    assert_eq!(
        serde_json::to_string(&ChannelType::DoubaoVideo).unwrap(),
        "\"DoubaoVideo\""
    );
    let parsed: ChannelType = serde_json::from_str("\"Anthropic\"").unwrap();
    assert_eq!(parsed, ChannelType::Anthropic);
    // Round trip over the whole listed range.
    for (_, variant) in LISTED {
        let json = serde_json::to_string(variant).unwrap();
        assert_eq!(
            serde_json::from_str::<ChannelType>(&json).unwrap(),
            *variant
        );
    }
}

#[test]
fn channel_serializes_type_as_type_not_type_underscore() {
    let channel = full_channel();
    let value: serde_json::Value = serde_json::to_value(&channel).unwrap();

    assert_eq!(
        value["type"],
        serde_json::json!(14),
        "the JSON name is `type`"
    );
    assert!(
        value.get("type_").is_none(),
        "the Rust field name must not leak into JSON"
    );
    // Spot-check a few fields that consumers read by name.
    assert_eq!(value["id"], serde_json::json!(42));
    assert_eq!(value["group"], serde_json::json!("default"));
    assert_eq!(
        value["models"],
        serde_json::json!("claude-3-5-sonnet,claude-3-haiku")
    );
    assert_eq!(value["used_quota"], serde_json::json!(1_234_567_890));
    assert_eq!(value["pricing_region"], serde_json::json!("international"));
    assert_eq!(value["reservation_green"], serde_json::json!(0.5));
    assert_eq!(value["rpm_cap"], serde_json::json!(600));
    // The credential field is serialized: the row must be able to hold it. Access control is the
    // caller's responsibility, not this contract's.
    assert_eq!(value["key"], serde_json::json!("sk-secret"));

    // Deserializing the produced JSON must give the same channel back, including `type`.
    let back: Channel = serde_json::from_value(value).unwrap();
    assert_eq!(back.type_, ChannelType::Anthropic as i32);
    assert_eq!(back.id, channel.id);
    assert_eq!(back.name, channel.name);
    assert_eq!(back.tpm_cap, channel.tpm_cap);
}

#[test]
fn optional_channel_fields_may_be_absent() {
    // Rows created before later columns existed have NULLs; the contract must accept that.
    let json = serde_json::json!({
        "id": 1,
        "type": 1,
        "key": "sk-x",
        "status": 1,
        "name": "minimal",
        "weight": 0,
        "created_time": null,
        "test_time": null,
        "response_time": null,
        "base_url": null,
        "models": "",
        "group": "default",
        "used_quota": 0,
        "model_mapping": null,
        "priority": 0,
        "auto_ban": 0,
        "other_info": null,
        "tag": null,
        "setting": null,
        "param_override": null,
        "header_override": null,
        "remark": null,
        "api_version": null,
        "pricing_region": null
    });
    let channel: Channel = serde_json::from_value(json).unwrap();
    assert_eq!(channel.type_, ChannelType::OpenAI as i32);
    assert!(channel.base_url.is_none());
    assert!(channel.model_mapping.is_none());
    // The shaper columns were added later and are optional in JSON.
    assert!(channel.rpm_cap.is_none());
    assert!(channel.reservation_red.is_none());
}

#[test]
fn ability_keeps_its_fields_and_enabled_flag() {
    let ability = Ability {
        group: "default".to_string(),
        model: "gpt-4o".to_string(),
        channel_id: 42,
        enabled: 1,
        priority: 5,
        weight: 3,
    };
    assert!(ability.enabled_bool());
    assert!(!Ability {
        enabled: 0,
        ..ability.clone()
    }
    .enabled_bool());

    let value: serde_json::Value = serde_json::to_value(&ability).unwrap();
    assert_eq!(value["group"], serde_json::json!("default"));
    assert_eq!(value["model"], serde_json::json!("gpt-4o"));
    assert_eq!(value["channel_id"], serde_json::json!(42));
    assert_eq!(value["enabled"], serde_json::json!(1));
    assert_eq!(value["priority"], serde_json::json!(5));
    assert_eq!(value["weight"], serde_json::json!(3));
    let back: Ability = serde_json::from_value(value).unwrap();
    assert_eq!(back.channel_id, 42);
}

#[test]
fn spec_aliases_name_the_same_types() {
    // The aliases are compatibility promises: `channel_abilities` / `channel_providers` row naming.
    let ability: ChannelAbility = Ability {
        group: "g".to_string(),
        model: "m".to_string(),
        channel_id: 1,
        enabled: 1,
        priority: 0,
        weight: 0,
    };
    let provider: ChannelProvider = full_channel();
    assert_eq!(ability.channel_id, 1);
    assert_eq!(provider.id, 42);
}
