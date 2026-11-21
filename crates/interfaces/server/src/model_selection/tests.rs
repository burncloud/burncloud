//! 使用本地站点替身验证统一接口，不访问公网。

#![allow(
    clippy::disallowed_types,
    reason = "HTTP 黑盒测试需要构造和检查站点协议响应"
)]

use super::*;
use axum::{
    body::to_bytes,
    extract::Query,
    http::{Request, Uri},
};
use serde_json::Value;
use std::{collections::HashMap, sync::Mutex};
use tower::ServiceExt;

type Calls = Arc<Mutex<Vec<String>>>;

struct Harness {
    router: Router,
    state: Arc<AppState>,
    calls: Calls,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn upstream(
    State(calls): State<Calls>,
    uri: Uri,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    calls.lock().expect("请求记录锁").push(uri.to_string());
    if uri.path().ends_with("/api/models/org/repo/tree/main") {
        return Json(json!([{"type":"file", "path":"model.gguf", "size":3, "lfs":{"oid":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}}])).into_response();
    }
    if !params.contains_key("search") {
        let status = if uri.path().contains("/missing/") {
            StatusCode::NOT_FOUND
        } else if uri.path().contains("/denied/") {
            StatusCode::FORBIDDEN
        } else {
            StatusCode::OK
        };
        return status.into_response();
    }
    let query = params["search"].as_str();
    let ms = uri.path().starts_with("/ms/");
    let hf = uri.path().starts_with("/hf/");
    if query == "timeout" && !hf {
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    if query == "normal" && ms {
        tokio::time::sleep(Duration::from_millis(60)).await;
    }
    if query == "failed" || (query == "partial" && ms) {
        return StatusCode::BAD_GATEWAY.into_response();
    }
    if query == "malformed" {
        return Json(json!({"unexpected":true})).into_response();
    }
    let models = if query == "empty" {
        json!([])
    } else {
        json!([{"id":"Qwen/Qwen3-8B", "title":if ms {"国内模型"} else {"镜像模型"}, "display_name":"国内模型"}])
    };
    if ms {
        Json(json!({"success":true,"data":{"models":models}})).into_response()
    } else {
        Json(models).into_response()
    }
}

impl Harness {
    async fn new() -> Self {
        let calls = Calls::default();
        let mock = Router::new().fallback(upstream).with_state(calls.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("监听测试站点");
        let base = format!("http://{}", listener.local_addr().expect("站点地址"));
        let config = || Config {
            modelscope: format!("{base}/ms"),
            hf_mirror: format!("{base}/mirror"),
            huggingface: format!("{base}/hf"),
            search_timeout: Duration::from_millis(250),
            probe_timeout: Duration::from_millis(250),
        };
        let client = reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("测试客户端");
        let router = app(config(), client.clone(), None);
        let state = Arc::new(AppState {
            config: config(),
            client,
            manifests: load_default_manifests().expect("默认模型清单"),
            database: None,
        });
        let task = tokio::spawn(async move {
            axum::serve(listener, mock).await.expect("测试站点运行");
        });
        Self {
            router,
            state,
            calls,
            task,
        }
    }

    async fn get(&self, path: &str) -> (StatusCode, String) {
        let response = self
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .body(Body::empty())
                    .expect("测试请求"),
            )
            .await
            .expect("测试路由");
        let status = response.status();
        let body = to_bytes(response.into_body(), 1_000_000)
            .await
            .expect("读取响应");
        (status, String::from_utf8(body.to_vec()).expect("响应编码"))
    }

    async fn json(&self, path: &str) -> Value {
        let (status, body) = self.get(path).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        serde_json::from_str(&body).expect("JSON 响应")
    }

    async fn search(&self, query: &str) -> Vec<Value> {
        let (status, body) = self.get(&format!("/api/models/search?q={query}")).await;
        assert_eq!(status, StatusCode::OK);
        body.lines()
            .map(|line| serde_json::from_str(line).expect("NDJSON 快照"))
            .collect()
    }
}

#[tokio::test]
async fn presets_keep_spreadsheet_rows_links_and_hardware() {
    let h = Harness::new().await;
    let presets = h.json("/api/models/presets").await;
    let names: Vec<_> = presets
        .as_array()
        .expect("预置列表")
        .iter()
        .map(|p| p["name"].as_str().expect("名称"))
        .collect();
    assert_eq!(names.len(), 388);
    assert_eq!(
        &names[..3],
        ["llama3.1-8B", "llama3.1-70B", "llama3.1-405B"]
    );
    assert_eq!(names[22], "qwen3-8B");
    assert_eq!(names.last(), Some(&"nimble-v3-9B"));
    assert_eq!(names.iter().filter(|name| **name == "llama2-7B").count(), 3);
    let items = presets.as_array().expect("列表");
    let mut ids: Vec<_> = items
        .iter()
        .map(|item| item["id"].as_str().expect("标识"))
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 388);
    assert_eq!(
        items
            .iter()
            .filter(|item| item["source"] == "modelscope")
            .count(),
        95
    );
    assert_eq!(
        items
            .iter()
            .filter(|item| item["source"] == "hf-mirror")
            .count(),
        293
    );
    assert_eq!(
        items[0]["resource_url"],
        "https://www.modelscope.cn/models/LLM-Research/Meta-Llama-3.1-8b-Instruct"
    );
    assert_eq!(
        items[0]["hardware"],
        json!({
            "vram_min":"6.00G", "ram_min":"12.00G", "disk_min":"6.00G",
            "vram_recommended":"8.00G", "ram_recommended":"16.00G", "disk_recommended":"8.00G"
        })
    );
    for item in items {
        assert_eq!(item["preset"], true);
        assert_eq!(
            item["revision"],
            if item["source"] == "modelscope" {
                "master"
            } else {
                "main"
            }
        );
        let link = reqwest::Url::parse(item["resource_url"].as_str().expect("模型链接"))
            .expect("有效链接");
        assert_eq!(link.scheme(), "https");
        let model = item["model"].as_str().expect("仓库");
        assert_eq!(
            link.path()
                .strip_prefix("/models/")
                .unwrap_or_else(|| link.path().trim_start_matches('/')),
            model
        );
        for field in [
            "vram_min",
            "ram_min",
            "disk_min",
            "vram_recommended",
            "ram_recommended",
            "disk_recommended",
        ] {
            assert!(item["hardware"][field]
                .as_str()
                .expect("配置")
                .strip_suffix('G')
                .is_some_and(|number| {
                    let decimals = number.split_once('.').map(|(_, decimals)| decimals);
                    decimals.is_some_and(|decimals| decimals.len() == 2)
                }));
        }
        let bound = presets::find(item["id"].as_str().expect("标识")).expect("绑定");
        assert_eq!(bound.resource.model, model);
    }
    assert!(h.calls.lock().expect("请求锁").is_empty());
    let response = routes(Arc::new(
        burncloud_database::Database::new()
            .await
            .expect("测试数据库"),
    ))
    .expect("生产路由")
    .oneshot(
        Request::builder()
            .uri("/api/models/presets")
            .body(Body::empty())
            .expect("请求"),
    )
    .await
    .expect("路由");
    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn gguf_repository_aliases_keep_resource_variants() {
    let manifests = default_manifests();
    for (model, source, revision) in [
        ("Qwen/Qwen3-8B-GGUF", Source::ModelScope, "master"),
        (
            "smalinin/DeepSeek-V4.1-Flash-GGUF",
            Source::HuggingFace,
            "main",
        ),
    ] {
        let resource = ModelResource {
            model: model.into(),
            source,
            revision: revision.into(),
        };
        let variants = variants_for_resource(&manifests, &resource);
        assert_eq!(variants.len(), 1, "GGUF 仓库缺少资源配置：{model}");
        assert!(variants[0].artifact_source.contains(model));
    }
}

#[tokio::test]
async fn status_uses_preset_binding_and_get_only_200_allows_handoff() {
    let h = Harness::new().await;
    let value = h
        .json("/api/models/status?preset=qwen3-8b&model=evil/override&source=huggingface")
        .await;
    assert_eq!(value["status"], "downloadable");
    assert_eq!(value["model"], "Qwen/Qwen3-8b");
    assert_eq!(value["source"], "modelscope");
    assert!(value["resource_url"]
        .as_str()
        .expect("资源地址")
        .ends_with("/ms/models/Qwen/Qwen3-8b/files?Revision=master"));
    assert_eq!(value["artifact_status"], "not_downloaded");
    assert_eq!(value["downloaded"], false);
    assert!(value["local_path"].is_null());
    assert!(value["actual_sha256"].is_null());
    assert_eq!(value["variants"][0]["artifact_status"], "not_downloaded");
    assert_eq!(value["variants"][0]["downloaded"], false);
    assert!(value["variants"][0]["local_path"].is_null());
    assert!(value["variants"][0]["actual_sha256"].is_null());
    assert_eq!(value["variants"][0]["id"], "qwen3-8b-q4");
    assert_eq!(value["variants"][0]["quantization"], "Q4");
    assert_eq!(value["variants"][0]["runtime"], "llama.cpp");
    assert_eq!(
        value["variants"][0]["ram_bytes"],
        16 * 1024 * 1024 * 1024u64
    );
    assert_eq!(
        value["variants"][0]["vram_bytes"],
        6 * 1024 * 1024 * 1024u64
    );
    assert_eq!(
        value["variants"][0]["disk_bytes"],
        6 * 1024 * 1024 * 1024u64
    );
    for model in ["owner/missing", "owner/denied"] {
        assert_eq!(
            h.json(&format!(
                "/api/models/status?model={model}&source=hf-mirror"
            ))
            .await["status"],
            "developing"
        );
    }
    let calls = h.calls.lock().expect("请求锁");
    assert_eq!(calls.len(), 3);
    assert!(calls.iter().all(|call| !call.contains("search=")));
}

#[tokio::test]
async fn status_attaches_imported_variants_to_matching_presets() {
    let h = Harness::new().await;
    for (id, has_variants) in [
        ("qwen3-8b", true),
        ("gpt-oss-20b", true),
        ("gpt-oss-120b", true),
        ("meta-llama-3.1-8b-instruct", false),
        ("qwen3.5-9b", false),
    ] {
        let value = h.json(&format!("/api/models/status?preset={id}")).await;
        let variants = value["variants"].as_array().expect("Variant 列表");
        assert_eq!(!variants.is_empty(), has_variants, "{id}");
    }
}

#[tokio::test]
async fn duplicate_repository_rows_keep_independent_preset_bindings() {
    let h = Harness::new().await;
    let presets = h.json("/api/models/presets").await;
    let duplicates: Vec<_> = presets
        .as_array()
        .expect("清单")
        .iter()
        .filter(|item| item["model"] == "ggml-org/Bespoke-Nimble-9B-v3-GGUF")
        .collect();
    assert_eq!(duplicates.len(), 2);
    assert_ne!(duplicates[0]["id"], duplicates[1]["id"]);
    assert_eq!(duplicates[0]["name"], "flint-9B");
    assert_eq!(duplicates[1]["name"], "nimble-v3-9B");
    for preset in duplicates {
        let id = preset["id"].as_str().expect("标识");
        let value = h.json(&format!("/api/models/status?preset={id}")).await;
        assert_eq!(value["model"], preset["model"]);
        assert_eq!(value["source"], "hf-mirror");
        assert_eq!(value["revision"], "main");
        assert!(value["resource_url"]
            .as_str()
            .expect("地址")
            .ends_with("/mirror/ggml-org/Bespoke-Nimble-9B-v3-GGUF/tree/main"));
    }
}

#[tokio::test]
async fn invalid_inputs_never_reach_upstream() {
    let h = Harness::new().await;
    for path in [
        "/api/models/search",
        "/api/models/search?q=%20",
        "/api/models/status",
        "/api/models/status?preset=unknown",
        "/api/models/status?model=a/b&source=unknown",
        "/api/models/status?model=../x&source=modelscope",
        "/api/models/status?model=a/b/c&source=modelscope",
        "/api/models/status?model=a/b&source=modelscope&revision=..",
    ] {
        assert_eq!(h.get(path).await.0, StatusCode::BAD_REQUEST, "{path}");
    }
    assert!(h.calls.lock().expect("请求锁").is_empty());
}

#[tokio::test]
async fn search_streams_before_slow_source_and_replaces_by_priority() {
    let h = Harness::new().await;
    let (tx, mut rx) = mpsc::channel(4);
    let task = tokio::spawn(run_search(h.state.clone(), "normal".into(), tx));
    let first: Value =
        serde_json::from_str(&rx.recv().await.expect("首帧").expect("序列化")).expect("JSON");
    assert_eq!(first["done"], false);
    assert_eq!(first["items"][0]["source"], "hf-mirror");
    let mut lines = vec![first];
    while let Some(line) = rx.recv().await {
        lines.push(serde_json::from_str::<Value>(&line.expect("序列化")).expect("JSON"));
    }
    task.await.expect("搜索完成");
    assert_eq!(lines.len(), 3);
    let last = lines.last().expect("最终帧");
    assert_eq!(last["done"], true);
    assert_eq!(last["type"], "models");
    assert_eq!(last["items"].as_array().expect("命中项").len(), 1);
    assert_eq!(last["items"][0]["source"], "modelscope");
}

#[tokio::test]
async fn only_domestic_timeout_starts_a_single_fallback() {
    for (query, kind, fallback) in [
        ("timeout", "models", 1),
        ("empty", "unavailable", 0),
        ("failed", "failed", 0),
        ("partial", "models", 0),
        ("malformed", "failed", 0),
    ] {
        let h = Harness::new().await;
        let lines = h.search(query).await;
        let last = lines.last().expect("完成帧");
        assert_eq!(last["type"], kind, "{query}");
        assert_eq!(last["done"], true);
        assert_eq!(
            h.calls
                .lock()
                .expect("请求锁")
                .iter()
                .filter(|call| call.starts_with("/hf/"))
                .count(),
            fallback,
            "{query}"
        );
        if query == "timeout" {
            assert_eq!(last["items"][0]["source"], "huggingface");
        }
    }
}

#[tokio::test]
async fn dropping_search_receiver_cancels_inflight_requests() {
    let h = Harness::new().await;
    let (tx, rx) = mpsc::channel(4);
    let task = tokio::spawn(run_search(h.state.clone(), "timeout".into(), tx));
    tokio::time::timeout(Duration::from_secs(1), async {
        while h.calls.lock().expect("请求锁").len() < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("国内请求已启动");
    drop(rx);
    tokio::time::timeout(Duration::from_millis(150), task)
        .await
        .expect("立即取消而非等待超时")
        .expect("搜索任务");
    tokio::time::timeout(Duration::from_millis(150), async {
        while Arc::strong_count(&h.state) != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("子任务已释放状态");
    assert_eq!(h.calls.lock().expect("请求锁").len(), 2);
}

#[tokio::test]
async fn unreachable_resource_is_not_downloadable() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("测试端口");
    let base = format!("http://{}", listener.local_addr().expect("地址"));
    drop(listener);
    let config = Config {
        modelscope: base,
        probe_timeout: Duration::from_millis(50),
        ..Config::default()
    };
    let response = app(config, reqwest::Client::new(), None)
        .oneshot(
            Request::builder()
                .uri("/api/models/status?preset=qwen3-8b")
                .body(Body::empty())
                .expect("请求"),
        )
        .await
        .expect("状态响应");
    let body = to_bytes(response.into_body(), 10000)
        .await
        .expect("读取响应");
    assert_eq!(
        serde_json::from_slice::<Value>(&body).expect("JSON")["status"],
        "developing"
    );
}

#[tokio::test]
async fn catalog_survives_restart_and_matches_current_remote_artifact() {
    let mut h = Harness::new().await;
    let dir = tempfile::tempdir().expect("临时目录");
    let path = dir.path().join("model.gguf");
    tokio::fs::write(&path, b"abc").await.expect("工件");
    let file = dir.path().join("catalog.db");
    let url = burncloud_database::sqlite_url(
        &file.to_string_lossy().replace('\\', "/"),
        cfg!(windows),
        true,
    );
    let digest = "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let db = burncloud_database::create_database_with_url(&url)
        .await
        .expect("建库");
    db.execute_query_with_params("INSERT INTO sys_downloads (gid, uris, status, model, source, revision, variant_id, artifact_uri, checksum, actual_sha256, final_path, total_length, completed_length) VALUES ('test', '[]', 'succeeded', 'org/repo', 'huggingface', 'main', 'model.gguf', 'https://huggingface.co/org/repo/resolve/main/model.gguf', ?, ?, ?, 3, 3)", vec![digest.into(), digest.into(), path.to_string_lossy().into_owned()]).await.expect("写入校验凭据");
    drop(db);
    let db = Arc::new(
        burncloud_database::create_database_with_url(&url)
            .await
            .expect("重新连接同一数据库"),
    );
    let config = Config {
        huggingface: h.state.config.huggingface.clone(),
        ..Config::default()
    };
    h.router = app(config, h.state.client.clone(), Some(db.clone()));
    let query = "/api/models/status?model=org/repo&source=huggingface&revision=main";
    for (update, expected) in [
        ("", "downloaded"),
        ("UPDATE sys_downloads SET status='active'", "downloading"),
        ("UPDATE sys_downloads SET status='completed'", "verifying"),
        ("UPDATE sys_downloads SET status='succeeded', actual_sha256=NULL", "failed"),
        ("UPDATE sys_downloads SET actual_sha256=checksum, final_path='nonexistent-file'", "failed"),
        ("UPDATE sys_downloads SET checksum='sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'", "not_downloaded"),
    ] {
        if !update.is_empty() { db.execute_query_with_params(update, Vec::new()).await.expect("更新测试记录"); }
        let (code, body) = h.get(query).await;
        assert_eq!(code, StatusCode::OK);
        let body: Value = serde_json::from_str(&body).expect("目录响应");
        assert_eq!(body["status"], "downloadable");
        assert_eq!(body["artifact_status"], expected);
        assert_eq!(body["variants"][0]["artifact_status"], expected);
        assert_eq!(body["downloaded"], expected == "downloaded");
        assert_eq!(body["actual_sha256"].as_str(), (expected == "downloaded").then_some(digest));
    }
}
