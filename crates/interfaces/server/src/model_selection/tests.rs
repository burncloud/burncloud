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
        let router = app(config(), client.clone());
        let state = Arc::new(AppState {
            config: config(),
            client,
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
async fn presets_keep_all_names_order_and_bound_resources() {
    let h = Harness::new().await;
    let presets = h.json("/api/models/presets").await;
    let names: Vec<_> = presets
        .as_array()
        .expect("预置列表")
        .iter()
        .map(|p| p["name"].as_str().expect("名称"))
        .collect();
    assert_eq!(
        names,
        [
            "DeepSeek V4.1 Flash",
            "DeepSeek V4 Pro",
            "GLM‑5.3",
            "GLM‑5.3‑Flash",
            "Kimi K3",
            "MiniMax M3.1 Flash",
            "Qwen3.8-27B",
            "Qwen3-8B",
            "Qwen3-Coder-Next",
            "Qwen3.8-2.4T-A95B",
            "Qwen3.8-Flash-Next",
            "Qwen-Image-2512",
            "Wan2.2-TI2V-5B",
            "MiMo-V2.6-Flash-RL",
            "gpt-oss-20b",
            "gpt-oss-120b",
            "Gemma 4 E4B IT",
            "Gemma 4 31B"
        ]
    );
    let items = presets.as_array().expect("列表");
    let mut ids: Vec<_> = items
        .iter()
        .map(|item| item["id"].as_str().expect("标识"))
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 18);
    assert!(items.iter().all(|item| item["source"] == "modelscope"
        && item["revision"] == "master"
        && item["preset"] == true));
    assert!(h.calls.lock().expect("请求锁").is_empty());
    let response = routes()
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

#[tokio::test]
async fn status_uses_preset_binding_and_get_only_200_allows_handoff() {
    let h = Harness::new().await;
    let value = h
        .json("/api/models/status?preset=qwen3-8b&model=evil/override&source=huggingface")
        .await;
    assert_eq!(value["status"], "downloadable");
    assert_eq!(value["model"], "Qwen/Qwen3-8B");
    assert_eq!(value["source"], "modelscope");
    assert!(value["resource_url"]
        .as_str()
        .expect("资源地址")
        .ends_with("/ms/models/Qwen/Qwen3-8B/files?Revision=master"));
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
    let response = app(config, reqwest::Client::new())
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
