use super::*;
use serde_json::json;

#[test]
fn validates_model_ids() {
    assert!(is_valid_model_id("Qwen/Qwen3-8B"));
    assert!(is_valid_model_id("google/gemma-4-E4B-it"));
    assert!(!is_valid_model_id("Qwen3-8B"));
    assert!(!is_valid_model_id("a/b/c"));
    assert!(!is_valid_model_id("../etc"));
    assert!(!is_valid_model_id("a/.."));
    assert!(!is_valid_model_id("a/b?x=1"));
}

#[test]
fn builds_resource_urls() {
    let config = Config::default();
    let ms = ModelResource {
        model: "Qwen/Qwen3-8B".into(),
        source: Source::ModelScope,
        revision: "master".into(),
    };
    assert_eq!(
        ms.url(&config).expect("固定测试数据应有效").as_str(),
        "https://modelscope.cn/models/Qwen/Qwen3-8B/files?Revision=master"
    );
    let hf = ModelResource {
        source: Source::HfMirror,
        revision: "main".into(),
        ..ms.clone()
    };
    assert_eq!(
        hf.url(&config).expect("固定测试数据应有效").as_str(),
        "https://hf-mirror.com/Qwen/Qwen3-8B/tree/main"
    );
    let bad = ModelResource {
        model: "../x".into(),
        ..ms
    };
    assert!(bad.url(&config).is_none());
}

#[test]
fn parses_modelscope_titles() {
    let payload = json!({"success": true, "data": {"models": [
        {"id": "Qwen/Qwen3-8B", "display_name": "千问3-8B"},
        {"id": "org/no-title"},
        {"id": "invalid"}
    ]}});
    let items = parse_modelscope(&payload).expect("固定测试数据应有效");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].name, "千问3-8B");
    assert_eq!(items[0].id, "qwen3-8b");
    assert_eq!(items[0].resource.revision, "master");
    assert_eq!(items[1].name, "no title");
    assert!(parse_modelscope(&json!({"success": false})).is_none());
}

#[test]
fn parses_hf_titles() {
    let payload = json!([
        {"id": "Qwen/Qwen3-4B", "cardData": {"model_name": "Qwen3 4B"}},
        {"modelId": "org/plain_model"}
    ]);
    let items = parse_hf(&payload, Source::HfMirror).expect("固定测试数据应有效");
    assert_eq!(items[0].name, "Qwen3 4B");
    assert_eq!(items[0].resource.revision, "main");
    assert_eq!(items[1].name, "plain model");
    assert!(parse_hf(&json!({"error": "x"}), Source::HfMirror).is_none());
}

#[test]
fn merge_prefers_modelscope_and_keeps_order() {
    let item = |model: &str, name: &str, source| SearchItem {
        id: version_id(model),
        name: name.into(),
        resource: ModelResource {
            model: model.into(),
            source,
            revision: "main".into(),
        },
    };
    let mut merged = Merged::default();
    assert!(merged.add(vec![
        item("Qwen/Qwen3-8B", "mirror", Source::HfMirror),
        item("Qwen/Qwen3-4B", "4b", Source::HfMirror),
    ]));
    assert!(merged.add(vec![item("Qwen/Qwen3-8B", "ms", Source::ModelScope)]));
    assert!(!merged.add(vec![item("Qwen/Qwen3-8B", "hf", Source::HuggingFace)]));
    let names: Vec<_> = merged.items().iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["ms", "4b"]);
}

#[test]
fn search_urls_per_source() {
    let config = Config::default();
    let ms = search_url(&config, Source::ModelScope, "qwen 3").expect("固定测试数据应有效");
    assert_eq!(ms.path(), "/openapi/v1/models");
    assert!(ms
        .query()
        .expect("固定测试数据应有效")
        .contains("search=qwen+3"));
    let hf = search_url(&config, Source::HuggingFace, "q").expect("固定测试数据应有效");
    assert_eq!(hf.origin().ascii_serialization(), "https://huggingface.co");
    assert!(hf
        .query()
        .expect("固定测试数据应有效")
        .contains("cardData=true"));
}
