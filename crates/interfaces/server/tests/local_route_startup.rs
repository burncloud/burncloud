#![expect(
    clippy::disallowed_types,
    reason = "仅在受控 HTTP 请求/响应协议边界使用动态 JSON，不用于生产业务模型"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic_in_result_fn,
    reason = "独立进程集成测试使用断言作为失败信号，不放宽生产错误处理"
)]

use axum::{
    body::{Body, Bytes},
    http::HeaderMap,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use burncloud_database::Database;
use burncloud_router::{
    local_attachment::{ExistingRouterLocalAttacher, LocalRouteAttacher, LocalRouteAttachmentId},
    price_sync::PriceSyncService,
};
use burncloud_supply_channel::ChannelService;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::time::{sleep, timeout};

const MODEL: &str = "Qwen:4b";
const LATER: &str = "Later:4b";
const FRESH: &str = "Fresh:4b";
const DEPLOYED: &str = "local-small-model";

struct Fixture {
    dir: tempfile::TempDir,
    db: Arc<Database>,
    url: String,
}

impl Fixture {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let url = burncloud_database::sqlite_url(
            &dir.path().join("server.db").to_string_lossy(),
            cfg!(windows),
            true,
        );
        let db = Arc::new(
            burncloud_database::create_database_with_url(&url)
                .await
                .unwrap(),
        );
        // 用现有价格导入能力准备夹具，不关闭真实认证/计费、不新增依赖。
        let config = serde_json::from_value(json!({
            "updated_at":"2026-11-05T00:00:00Z", "source":"local-startup-test",
            "models": {
                MODEL: {"pricing":{"USD":{"input_price":1.0,"output_price":1.0}}},
                LATER: {"pricing":{"USD":{"input_price":1.0,"output_price":1.0}}},
                FRESH: {"pricing":{"USD":{"input_price":1.0,"output_price":1.0}}},
                DEPLOYED: {"pricing":{"USD":{"input_price":1.0,"output_price":1.0}}}
            }
        }))
        .unwrap();
        let result = PriceSyncService::new(db.clone())
            .apply_prices(&config, "local-startup-test")
            .await
            .unwrap();
        assert_eq!(result.errors, 0);
        assert_eq!(result.models_synced, 4);
        Self { dir, db, url }
    }

    fn start(&self, config: Option<&str>) -> ServerProcess {
        self.start_options(config, None, None, "{}")
    }

    fn start_options(
        &self,
        config: Option<&str>,
        deployments: Option<&str>,
        concurrency: Option<&str>,
        policies: &str,
    ) -> ServerProcess {
        // start_server 不暴露 listener；使用系统分配端口并在移交前释放，绑定失败由子进程日志报告。
        let reserved = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = reserved.local_addr().unwrap().port();
        let stop = self.dir.path().join(format!("stop-{port}"));
        let log = self.dir.path().join(format!("log-{port}"));
        let output = std::fs::File::create(&log).unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--ignored",
                "--exact",
                "local_route_server_worker",
                "--nocapture",
            ])
            .env("LOCAL_ROUTE_TEST_PORT", port.to_string())
            .env("LOCAL_ROUTE_TEST_STOP", &stop)
            .env("BURNCLOUD_DATABASE_URL", &self.url)
            .env("JWT_SECRET", "local-startup-jwt-secret")
            .env("BURNCLOUD_INTERNAL_SECRET", "local-startup-internal-secret")
            .env(
                "MASTER_KEY",
                "a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8",
            )
            .env("SKIP_INITIAL_PRICE_SYNC", "1")
            .env("PRICE_SYNC_INTERVAL_SECS", "86400")
            .env("SCHEDULER_POLICIES", policies)
            .env_remove("BURNCLOUD_LOCAL_ROUTE_MAX_CONCURRENCY")
            .env("BILLING_STRICT_MODE", "true")
            .env("REQUEST_LOG_STORAGE_POLICY", "summary")
            .env_remove("BURNCLOUD_NODE_TEST_API")
            .env_remove("BURNCLOUD_FRESH_DB")
            .env_remove("BURNCLOUD_LOCAL_ROUTE_ATTACHMENTS")
            .env_remove("BURNCLOUD_LOCAL_ROUTE_DEPLOYMENTS")
            .stdout(Stdio::from(output.try_clone().unwrap()))
            .stderr(Stdio::from(output));
        if let Some(limit) = concurrency {
            command.env("BURNCLOUD_LOCAL_ROUTE_MAX_CONCURRENCY", limit);
        }
        if let Some(config) = config {
            command.env("BURNCLOUD_LOCAL_ROUTE_ATTACHMENTS", config);
        }
        if let Some(deployments) = deployments {
            command.env("BURNCLOUD_LOCAL_ROUTE_DEPLOYMENTS", deployments);
        }
        drop(reserved);
        ServerProcess {
            child: command.spawn().unwrap(),
            base: format!("http://127.0.0.1:{port}"),
            stop,
            log,
        }
    }

    async fn wait_for_logged(&self, model: &str) {
        // 现有非流式成功路径只写汇总日志；等待该实际写入完成再显式清理，避免 SQLite 写争用。
        timeout(Duration::from_secs(10), async {
            loop {
                let logs = burncloud_router::RouterLogModel::get(&self.db, 100, 0)
                    .await
                    .unwrap();
                if logs
                    .iter()
                    .any(|log| log.model.as_deref() == Some(model) && log.status_code == 200)
                {
                    break;
                }
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
    }

    async fn wait_for_count(&self, count: usize) {
        timeout(Duration::from_secs(10), async {
            loop {
                if ChannelService::list(&self.db, 100, 0).await.unwrap().len() == count {
                    break;
                }
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
    }
}

struct ServerProcess {
    child: Child,
    base: String,
    stop: PathBuf,
    log: PathBuf,
}

impl ServerProcess {
    fn logs(&self) -> String {
        std::fs::read_to_string(&self.log).unwrap()
    }

    async fn ready(&mut self) {
        let client = client();
        timeout(Duration::from_secs(30), async {
            loop {
                assert!(
                    self.child.try_wait().unwrap().is_none(),
                    "server exited: {}",
                    self.logs()
                );
                if client
                    .get(format!("{}/health", self.base))
                    .send()
                    .await
                    .is_ok_and(|r| r.status().is_success())
                {
                    break;
                }
                sleep(Duration::from_millis(30)).await;
            }
        })
        .await
        .unwrap_or_else(|error| panic!("{error}: {}", self.logs()));
    }

    async fn wait_exit(&mut self, success: bool) {
        let status = timeout(Duration::from_secs(30), async {
            loop {
                if let Some(status) = self.child.try_wait().unwrap() {
                    break status;
                }
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap_or_else(|error| panic!("{error}: {}", self.logs()));
        assert_eq!(status.success(), success, "{}", self.logs());
    }

    async fn stop(&mut self) {
        std::fs::write(&self.stop, "stop").unwrap();
        self.wait_exit(true).await;
    }

    async fn wait_for_log(&self, marker: &str, count: usize) {
        timeout(Duration::from_secs(10), async {
            while self.logs().matches(marker).count() < count {
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap_or_else(|error| panic!("{error}: {}", self.logs()));
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        // 只回收本测试创建的子进程，异常退出也不留下后台 Server。
        if self.child.try_wait().expect("read child status").is_none() {
            self.child.kill().expect("stop owned child");
            self.child.wait().expect("reap owned child");
        }
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap()
}

async fn chat(server: &ServerProcess, model: &str) -> reqwest::Response {
    client()
        .post(format!("{}/v1/chat/completions", server.base))
        .bearer_auth("sk-burncloud-demo")
        .json(&json!({"model":model,"messages":[{"role":"user","content":"hello"}]}))
        .send()
        .await
        .unwrap()
}

fn config(url: &str, models: &[&str]) -> String {
    serde_json::to_string(
        &models
            .iter()
            .map(|model| json!({"model":model,"base_url":url}))
            .collect::<Vec<_>>(),
    )
    .unwrap()
}

struct ModelServer {
    url: String,
    ready: Arc<AtomicBool>,
    probes: Arc<AtomicUsize>,
    chats: Arc<AtomicUsize>,
    release_stream: Arc<tokio::sync::Notify>,
    task: tokio::task::JoinHandle<()>,
}

impl ModelServer {
    async fn new() -> Self {
        let ready = Arc::new(AtomicBool::new(false));
        let probes = Arc::new(AtomicUsize::new(0));
        let chats = Arc::new(AtomicUsize::new(0));
        let state = ready.clone();
        let counter = probes.clone();
        let chat_counter = chats.clone();
        let release_stream = Arc::new(tokio::sync::Notify::new());
        let stream_gate = release_stream.clone();
        let app = Router::new().route("/v1/models", get(move || {
            counter.fetch_add(1, Ordering::SeqCst);
            let ready = state.load(Ordering::SeqCst);
            async move {
                let mut data = vec![json!({"id":MODEL}), json!({"id":FRESH})];
                if ready { data.extend([json!({"id":LATER}), json!({"id":DEPLOYED})]); }
                Json(json!({"data":data}))
            }
        })).route("/v1/chat/completions", post(move |headers: HeaderMap, Json(body): Json<Value>| {
            assert!(!headers.contains_key("authorization"));
            chat_counter.fetch_add(1, Ordering::SeqCst);
            let stream_gate = stream_gate.clone();
            async move {
                if body.get("stream").and_then(Value::as_bool) == Some(true) {
                    // 把上游 EOF 控制在测试手中，验证返回响应头后仍占用挂载槽位。
                    let first = futures_util::stream::once(async {
                        Ok::<_, std::convert::Infallible>(Bytes::from_static(b"data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"controlled stream\"},\"finish_reason\":null}]}\n\n"))
                    });
                    let last = futures_util::stream::once(async move {
                        stream_gate.notified().await;
                        Ok::<_, std::convert::Infallible>(Bytes::from_static(b"data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":2,\"completion_tokens\":3,\"total_tokens\":5}}\n\ndata: [DONE]\n\n"))
                    });
                    return ([("content-type", "text/event-stream")], Body::from_stream(first.chain(last))).into_response();
                }
                Json(json!({"id":"controlled-response","object":"chat.completion","model":body["model"],
                    "choices":[{"index":0,"message":{"role":"assistant","content":"controlled local response"},"finish_reason":"stop"}],
                    "usage":{"prompt_tokens":2,"completion_tokens":3,"total_tokens":5}})).into_response()
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            url,
            ready,
            probes,
            chats,
            release_stream,
            task,
        }
    }
}

impl Drop for ModelServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[tokio::test]
#[ignore = "由父测试以独立环境变量启动的实际 start_server 子进程"]
async fn local_route_server_worker() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_ansi(false)
        .init();
    let port = std::env::var("LOCAL_ROUTE_TEST_PORT")?.parse()?;
    let stop = PathBuf::from(std::env::var("LOCAL_ROUTE_TEST_STOP")?);
    tokio::select! {
        result = burncloud_server::start_server("127.0.0.1", port, false) => result,
        () = async { while !stop.exists() { sleep(Duration::from_millis(10)).await; } } => Ok(()),
    }
}

#[tokio::test]
async fn normal_server_accepts_absent_and_empty_config_and_rejects_invalid_whole_lists() {
    let fixture = Fixture::new().await;
    let model = ModelServer::new().await;
    for raw in [None, Some("[]")] {
        let mut server = fixture.start(raw);
        server.ready().await;
        assert!(ChannelService::list(&fixture.db, 100, 0)
            .await
            .unwrap()
            .is_empty());
        server.stop().await;
    }
    let valid = json!({"model":MODEL, "base_url":model.url});
    let invalid_lists = [
        "".into(),
        "null".into(),
        "{}".into(),
        json!([valid, {"model":LATER}]).to_string(),
        json!([valid, {"model":LATER,"base_url":model.url,"heartbeat":1}]).to_string(),
        json!([valid, {"model":" bad","base_url":model.url}]).to_string(),
        json!([valid, {"model":LATER,"base_url":"http://localhost"}]).to_string(),
        json!([valid, {"model":MODEL,"base_url":format!("{}/",model.url)}]).to_string(),
    ];
    for raw in invalid_lists {
        let mut server = fixture.start(Some(&raw));
        server.wait_exit(false).await;
        assert!(server.logs().contains("Error:"), "{}", server.logs());
        assert!(ChannelService::list(&fixture.db, 100, 0)
            .await
            .unwrap()
            .is_empty());
    }
    assert_eq!(model.probes.load(Ordering::SeqCst), 0);
    fixture.db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn normal_server_routes_ready_model_retries_missing_alias_and_keeps_restart_residue_closed() {
    let fixture = Fixture::new().await;
    let model = ModelServer::new().await;
    let raw = config(&model.url, &[MODEL, LATER]);
    let mut server = fixture.start(Some(&raw));
    server.ready().await;
    fixture.wait_for_count(1).await;
    server
        .wait_for_log("local route startup attachment published", 1)
        .await;
    let response = chat(&server, MODEL).await;
    assert_eq!(response.status(), 200, "{}", response.text().await.unwrap());
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["model"], MODEL);
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "controlled local response"
    );
    model.ready.store(true, Ordering::SeqCst);
    fixture.wait_for_count(2).await;
    server
        .wait_for_log("local route startup attachment published", 2)
        .await;
    let response = chat(&server, LATER).await;
    assert_eq!(response.status(), 200, "{}", response.text().await.unwrap());
    let old = ChannelService::list(&fixture.db, 100, 0).await.unwrap();
    server.stop().await;

    // 同一 SQLite 文件、全新进程：旧挂载不得收养，无残留的新别名仍正常挂载。
    let raw = config(&model.url, &[MODEL, LATER, FRESH]);
    let mut restarted = fixture.start(Some(&raw));
    restarted.ready().await;
    fixture.wait_for_count(3).await;
    restarted
        .wait_for_log("local route automatic attachment stopped", 2)
        .await;
    let before = model.chats.load(Ordering::SeqCst);
    let rejected = chat(&restarted, MODEL).await;
    assert_eq!(rejected.status(), 503, "{}", rejected.text().await.unwrap());
    assert_eq!(
        rejected.headers().get("x-rejected-by").unwrap(),
        "scheduler"
    );
    assert_eq!(model.chats.load(Ordering::SeqCst), before);
    let accepted = chat(&restarted, FRESH).await;
    assert_eq!(accepted.status(), 200, "{}", accepted.text().await.unwrap());
    accepted.bytes().await.unwrap();
    fixture.wait_for_logged(FRESH).await;
    let original = old.iter().find(|channel| channel.models == MODEL).unwrap();
    assert!(ChannelService::get_by_id(&fixture.db, original.id)
        .await
        .unwrap()
        .is_some());
    ExistingRouterLocalAttacher::new(fixture.db.clone())
        .detach(LocalRouteAttachmentId(original.id))
        .await
        .unwrap();
    let probes = model.probes.load(Ordering::SeqCst);
    sleep(Duration::from_millis(5200)).await;
    assert_eq!(
        ChannelService::list(&fixture.db, 100, 0)
            .await
            .unwrap()
            .len(),
        2
    );
    // 运行中已挂载目标持续每 5 秒探测；detach 后该目标停止探测，残留项仍不可收养。
    assert!(model.probes.load(Ordering::SeqCst) >= probes);
    restarted.stop().await;
    fixture.db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn normal_server_does_not_retry_failed_attach_after_database_recovers() {
    let fixture = Fixture::new().await;
    let model = ModelServer::new().await;
    model.ready.store(true, Ordering::SeqCst);
    // 仅在独立 SQLite 测试库注入事务失败；生产 SQL 仍由 Supply 所有。
    fixture.db.execute_query("CREATE TRIGGER fail_local_startup BEFORE INSERT ON channel_providers WHEN NEW.models = 'Later:4b' BEGIN SELECT RAISE(FAIL, 'injected startup failure'); END;").await.unwrap();
    let raw = config(&model.url, &[MODEL, LATER]);
    let mut server = fixture.start(Some(&raw));
    server.ready().await;
    fixture.wait_for_count(1).await;
    server
        .wait_for_log("local route automatic attachment stopped", 1)
        .await;
    fixture
        .db
        .execute_query("DROP TRIGGER fail_local_startup;")
        .await
        .unwrap();
    sleep(Duration::from_millis(5200)).await;
    // MODEL 的运行心跳会再次探测；LATER 的 attach 失败项不会自动重试写入。
    assert_eq!(model.probes.load(Ordering::SeqCst), 3);
    assert_eq!(
        ChannelService::list(&fixture.db, 100, 0)
            .await
            .unwrap()
            .len(),
        1
    );
    let response = chat(&server, MODEL).await;
    assert_eq!(response.status(), 200, "{}", response.text().await.unwrap());
    server.stop().await;
    fixture.db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn normal_server_validates_capacity_and_policies_before_any_probe() {
    let fixture = Fixture::new().await;
    let model = ModelServer::new().await;
    let config = json!([{"model":MODEL,"base_url":model.url}]).to_string();
    for limit in ["0", "-1", "+1", " 1", "", "4294967296"] {
        let mut server = fixture.start_options(Some(&config), None, Some(limit), "{}");
        server.wait_exit(false).await;
        assert!(server
            .logs()
            .contains("BURNCLOUD_LOCAL_ROUTE_MAX_CONCURRENCY"));
    }
    for policies in [
        "null",
        r#"{"default":{"type":"unknown"}}"#,
        r#"{"A":{"type":"balanced"},"a":{"type":"price_first"}}"#,
        r#"{"default":{"type":"balanced","cost_weight":1}}"#,
    ] {
        let mut server = fixture.start_options(Some(&config), None, None, policies);
        server.wait_exit(false).await;
    }
    assert_eq!(model.probes.load(Ordering::SeqCst), 0);
    assert!(ChannelService::list(&fixture.db, 100, 0)
        .await
        .unwrap()
        .is_empty());
    let mut server = fixture.start_options(
        None,
        None,
        Some("4294967295"),
        r#"{"default":{"type":"balanced"}}"#,
    );
    server.ready().await;
    server.stop().await;
}

fn deployment_result(url: &str, model: &str) -> Value {
    json!({
        "model_id": model,
        "status": "ready",
        "stage": "HealthChecking",
        "state": "Ready",
        "endpoint": url,
        "pid": 18659,
        "last_heartbeat_unix_ms": 1793783953442_i64,
        "error": null,
        "events": [
            {"kind":"progress","stage":"Queued"},
            {"kind":"progress","stage":"Planning"},
            {"kind":"progress","stage":"Starting"},
            {"kind":"progress","stage":"WaitingReady"},
            {"kind":"progress","stage":"HealthChecking"},
            {"kind":"initial_result","state":"Ready"},
            {"kind":"heartbeat","state":"Ready","occurred_at_unix_ms":1793783953442_i64}
        ]
    })
}

#[tokio::test]
async fn deployment_receipt_is_reprobed_before_mount_and_streams_keep_capacity() {
    let fixture = Fixture::new().await;
    let model = ModelServer::new().await;
    let raw = deployment_result(&model.url, DEPLOYED).to_string();
    let mut server = fixture.start_options(None, Some(&raw), None, "{}");
    server.ready().await;
    server
        .wait_for_log("local route startup probe failed", 1)
        .await;
    assert!(ChannelService::list(&fixture.db, 100, 0)
        .await
        .unwrap()
        .is_empty());
    // 回执中的历史 Ready 与心跳不能代替当前端点中的精确模型声明。
    assert_eq!(chat(&server, DEPLOYED).await.status(), 404);
    assert_eq!(model.chats.load(Ordering::SeqCst), 0);

    model.ready.store(true, Ordering::SeqCst);
    fixture.wait_for_count(1).await;
    server
        .wait_for_log("local route startup attachment published", 1)
        .await;
    let channels = ChannelService::list(&fixture.db, 100, 0).await.unwrap();
    assert!(channels[0].id > 0);
    assert_eq!(channels[0].models, DEPLOYED);
    assert_eq!(channels[0].base_url.as_deref(), Some(model.url.as_str()));
    assert_eq!(channels[0].tag.as_deref(), Some("burncloud-node-local"));
    let response = chat(&server, DEPLOYED).await;
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["model"], DEPLOYED);
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "controlled local response"
    );

    let mut stream = client()
        .post(format!("{}/v1/chat/completions", server.base))
        .bearer_auth("sk-burncloud-demo")
        .json(
            &json!({"model":DEPLOYED,"stream":true,"messages":[{"role":"user","content":"hello"}]}),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(stream.status(), 200);
    assert_eq!(stream.headers()["content-type"], "text/event-stream");
    let first = stream.chunk().await.unwrap().unwrap();
    assert!(String::from_utf8_lossy(&first).contains("controlled stream"));
    let rejected = chat(&server, DEPLOYED).await;
    assert_eq!(rejected.status(), 503);
    assert_eq!(rejected.headers()["x-rejected-by"], "scheduler");
    assert_eq!(model.chats.load(Ordering::SeqCst), 2);
    model.release_stream.notify_one();
    assert!(stream.text().await.unwrap().contains("[DONE]"));
    let after_eof = chat(&server, DEPLOYED).await;
    assert_eq!(after_eof.status(), 200);
    after_eof.bytes().await.unwrap();
    assert_eq!(model.chats.load(Ordering::SeqCst), 3);
    server.stop().await;
    fixture.db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn deployment_arrays_validate_all_targets_and_conflicts_before_probing() {
    let fixture = Fixture::new().await;
    let model = ModelServer::new().await;
    let valid = deployment_result(&model.url, MODEL);
    let mut not_ready = deployment_result(&model.url, FRESH);
    not_ready["state"] = json!("Starting");
    let mut invalid_url = deployment_result("http://localhost:60544", FRESH);
    let invalid = [
        "".into(),
        "null".into(),
        "{}".into(),
        json!([valid, not_ready]).to_string(),
        json!([valid, invalid_url]).to_string(),
        json!([valid, deployment_result(&model.url, " local-small-model ")]).to_string(),
        json!([
            valid,
            deployment_result(&format!("[{}]({})", model.url, model.url), FRESH)
        ])
        .to_string(),
        json!([valid, deployment_result(&format!("{}/", model.url), MODEL)]).to_string(),
    ];
    for raw in invalid {
        let mut server = fixture.start_options(None, Some(&raw), None, "{}");
        server.wait_exit(false).await;
        assert!(server.logs().contains("Error:"), "{}", server.logs());
    }
    invalid_url["endpoint"] = json!(model.url);
    invalid_url["error"] = json!({"message":"deployment failed"});
    let raw = json!([valid, invalid_url]).to_string();
    let mut failed = fixture.start_options(None, Some(&raw), None, "{}");
    failed.wait_exit(false).await;
    let mut conflict = fixture.start_options(Some("[]"), Some("[]"), None, "{}");
    conflict.wait_exit(false).await;
    assert!(conflict
        .logs()
        .contains("BURNCLOUD_LOCAL_ROUTE_ATTACHMENTS"));
    assert!(conflict
        .logs()
        .contains("BURNCLOUD_LOCAL_ROUTE_DEPLOYMENTS"));
    let mut empty = fixture.start_options(None, Some("[]"), None, "{}");
    empty.ready().await;
    empty.stop().await;
    assert_eq!(model.probes.load(Ordering::SeqCst), 0);
    assert!(ChannelService::list(&fixture.db, 100, 0)
        .await
        .unwrap()
        .is_empty());
    fixture.db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn deployment_array_mounts_exact_targets_but_restart_cannot_adopt_residue() {
    let fixture = Fixture::new().await;
    let model = ModelServer::new().await;
    let raw = json!([
        deployment_result(&model.url, MODEL),
        deployment_result(&model.url, FRESH)
    ])
    .to_string();
    let mut server = fixture.start_options(None, Some(&raw), None, "{}");
    server.ready().await;
    fixture.wait_for_count(2).await;
    server
        .wait_for_log("local route startup attachment published", 2)
        .await;
    for name in [MODEL, FRESH] {
        let response = chat(&server, name).await;
        assert_eq!(response.status(), 200);
        assert_eq!(response.json::<Value>().await.unwrap()["model"], name);
        fixture.wait_for_logged(name).await;
    }
    let original = ChannelService::list(&fixture.db, 100, 0).await.unwrap();
    assert_ne!(original[0].id, original[1].id);
    server.stop().await;
    let chats = model.chats.load(Ordering::SeqCst);
    let mut restarted = fixture.start_options(None, Some(&raw), None, "{}");
    restarted.ready().await;
    restarted
        .wait_for_log("local route automatic attachment stopped", 2)
        .await;
    for name in [MODEL, FRESH] {
        assert_eq!(chat(&restarted, name).await.status(), 503);
    }
    assert_eq!(model.chats.load(Ordering::SeqCst), chats);
    assert_eq!(
        ChannelService::list(&fixture.db, 100, 0)
            .await
            .unwrap()
            .len(),
        2
    );
    restarted.stop().await;
    fixture.db.get_connection().unwrap().pool().close().await;
}
