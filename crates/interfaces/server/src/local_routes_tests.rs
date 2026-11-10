#![allow(clippy::unwrap_used, reason = "测试固定输入解码失败应立即中止")]
use super::*;

#[test]
fn absent_and_empty_lists_are_valid_but_present_invalid_values_are_not() {
    for raw in [None, Some("[]"), Some(" \n [] \t")] {
        assert!(parse_attachments(raw).is_ok_and(|items| items.is_empty()));
    }
    for raw in [
        "",
        " ",
        "null",
        "{}",
        "42",
        "[null]",
        "[{}]",
        r#"[{"model":"m","base_url":1}]"#,
        r#"[{"model":"m","base_url":"http://127.0.0.1","heartbeat":1}]"#,
        r#"[{"model":"m","model":"n","base_url":"http://127.0.0.1"}]"#,
    ] {
        assert!(parse_attachments(Some(raw)).is_err(), "accepted {raw}");
    }
}

#[test]
fn decoding_preserves_alias_and_url_for_the_shared_router_validator() {
    let items = parse_attachments(Some(
        r#"[{"model":"Qwen:中文 4b","base_url":"http://127.0.0.1:80/"}]"#,
    ))
    .unwrap();
    assert_eq!(
        items,
        vec![LocalRouteAttachment {
            model: "Qwen:中文 4b".into(),
            base_url: "http://127.0.0.1:80/".into(),
        }]
    );
}

const READY_DEPLOYMENT: &str = r#"{
    "model_id":"local-small-model", "status":"ready", "stage":"HealthChecking",
    "state":"Ready", "endpoint":"http://127.0.0.1:60544", "pid":18659,
    "last_heartbeat_unix_ms":1793783953442, "error":null,
    "events":[{"kind":"initial_result","state":"Ready"},
        {"kind":"heartbeat","state":"Ready","occurred_at_unix_ms":1793783953442}]
}"#;

#[test]
fn deployment_object_and_array_map_only_target_fields() {
    let expected = LocalRouteAttachment {
        model: "local-small-model".into(),
        base_url: "http://127.0.0.1:60544".into(),
    };
    assert_eq!(
        parse_sources(None, Some(READY_DEPLOYMENT)).unwrap(),
        vec![expected.clone()]
    );
    let another = READY_DEPLOYMENT.replace("local-small-model", "Qwen:中文 4b");
    let array = format!("[{READY_DEPLOYMENT},{another}]");
    let targets = parse_sources(None, Some(&array)).unwrap();
    assert_eq!(targets[0], expected);
    assert_eq!(targets[1].model, "Qwen:中文 4b");
    // 过去或未来的部署心跳均只作为被忽略的元数据，最终健康证据必须来自重新探测。
    for timestamp in ["0", "18446744073709551615"] {
        let raw = READY_DEPLOYMENT.replace("1793783953442", timestamp);
        assert_eq!(
            parse_sources(None, Some(&raw)).unwrap(),
            vec![expected.clone()]
        );
    }
    let raw = READY_DEPLOYMENT.replace("local-small-model", " not normalized ");
    assert_eq!(
        parse_sources(None, Some(&raw)).unwrap()[0].model,
        " not normalized "
    );
}

#[test]
fn deployment_status_and_errors_fail_closed_for_the_whole_list() {
    for (from, to) in [
        (r#""status":"ready""#, r#""status":"failed""#),
        (r#""status":"ready""#, r#""status":"Ready""#),
        (r#""state":"Ready""#, r#""state":"WaitingReady""#),
        (r#""state":"Ready""#, r#""state":"Routable""#),
        (r#""error":null"#, r#""error":{"message":"failure"}"#),
        (r#""error":null"#, r#""error":"""#),
        (r#""error":null"#, r#""error":false"#),
        (r#""error":null"#, r#""error":0"#),
        (r#""error":null"#, r#""error":[]"#),
    ] {
        let invalid = READY_DEPLOYMENT.replace(from, to);
        assert_ne!(invalid, READY_DEPLOYMENT);
        let error = parse_sources(None, Some(&invalid)).unwrap_err().to_string();
        assert!(error.contains("entry 0"), "{error}");
        let mixed = format!("[{READY_DEPLOYMENT},{invalid}]");
        assert!(parse_sources(None, Some(&mixed))
            .unwrap_err()
            .to_string()
            .contains("entry 1"));
    }
    let missing_error = READY_DEPLOYMENT.replace(r#""error":null,"#, "");
    assert_eq!(parse_sources(None, Some(&missing_error)).unwrap().len(), 1);
}

#[test]
fn deployment_shape_requires_exact_fields_and_rejects_duplicate_keys() {
    for raw in ["", " ", "null", "{}", "42", "[null]", "[{}]", "[[{}]]"] {
        assert!(parse_sources(None, Some(raw)).is_err(), "accepted {raw}");
    }
    for (from, to) in [
        (
            r#""model_id":"local-small-model""#,
            r#""model":"local-small-model""#,
        ),
        (r#""status":"ready""#, r#""status":null"#),
        (r#""state":"Ready""#, r#""missing_state":"Ready""#),
        (
            r#""endpoint":"http://127.0.0.1:60544""#,
            r#""endpoint":null"#,
        ),
        (r#""endpoint":"http://127.0.0.1:60544""#, r#""endpoint":42"#),
        (
            r#""status":"ready""#,
            r#""status":"failed","status":"ready""#,
        ),
        (
            r#""model_id":"local-small-model""#,
            r#""model_id":"other","model_id":"local-small-model""#,
        ),
    ] {
        let raw = READY_DEPLOYMENT.replace(from, to);
        assert_ne!(raw, READY_DEPLOYMENT);
        assert!(parse_sources(None, Some(&raw)).is_err(), "accepted {raw}");
    }
}

#[test]
fn startup_sources_are_exclusive_and_legacy_shape_is_unchanged() {
    for raw in [None, Some("[]")] {
        assert!(parse_sources(None, raw).unwrap().is_empty());
    }
    let legacy = r#"[{"model":"local-small-model","base_url":"http://127.0.0.1:60544"}]"#;
    assert_eq!(
        parse_sources(Some(legacy), None).unwrap(),
        parse_sources(None, Some(READY_DEPLOYMENT)).unwrap()
    );
    assert!(parse_sources(Some(READY_DEPLOYMENT), None).is_err());
    assert!(parse_sources(None, Some(legacy)).is_err());
    for (legacy, deployment) in [("[]", "[]"), ("", ""), (legacy, READY_DEPLOYMENT)] {
        let error = parse_sources(Some(legacy), Some(deployment))
            .unwrap_err()
            .to_string();
        assert!(error.contains("mutually exclusive"));
    }
}
