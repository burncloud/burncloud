#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "独立白盒测试的受控资源和断言失败必须中止"
)]
use super::LocalAttachmentState;
use crate::local_attachment::{
    ExistingRouterLocalAttacher, LocalRouteAttacher, LocalRouteAttachment,
};
use crate::*;
use burncloud_supply_channel::ChannelService;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{mpsc, Notify};

async fn seed_price(db: &burncloud_database::Database, model: &str) -> anyhow::Result<()> {
    use burncloud_commerce_billing::{BillingPriceModel, PriceInput};
    use burncloud_commerce_contracts::price_u64::dollars_to_nano;

    let input = PriceInput {
        model: model.to_string(),
        input_price: dollars_to_nano(1.0),
        output_price: dollars_to_nano(1.0),
        currency: "USD".to_string(),
        cache_read_input_price: None,
        cache_creation_input_price: None,
        batch_input_price: None,
        batch_output_price: None,
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        audio_output_price: None,
        reasoning_price: None,
        embedding_price: None,
        image_price: None,
        video_price: None,
        music_price: None,
        source: None,
        region: None,
        context_window: None,
        max_output_tokens: None,
        supports_vision: None,
        supports_function_calling: None,
        voices_pricing: None,
        video_pricing: None,
        asr_pricing: None,
        realtime_pricing: None,
        model_type: None,
    };
    BillingPriceModel::upsert(db, &input).await?;
    assert!(
        BillingPriceModel::get(db, model, "USD", None)
            .await?
            .is_some(),
        "seeded price must be readable through the production lookup"
    );
    Ok(())
}

struct Fixture {
    file: tempfile::NamedTempFile,
    db: Arc<Database>,
    state: AppState,
    logs: mpsc::Receiver<RouterLog>,
    _requests: mpsc::Receiver<RouterRequestLog>,
}

impl Fixture {
    async fn new(model: &str) -> Self {
        let file = tempfile::NamedTempFile::new().unwrap();
        let url =
            burncloud_database::sqlite_url(&file.path().to_string_lossy(), cfg!(windows), true);
        let db = Arc::new(
            burncloud_database::create_database_with_url(&url)
                .await
                .unwrap(),
        );
        let pool = db.get_connection().unwrap().pool();
        // 独立测试账户、密钥和价格仍经过真实认证与计费，不关闭生产校验。
        sqlx::query(r#"INSERT INTO user_accounts (id, username, password_hash, status, "group") VALUES ('local-test', 'local-test', '', 1, 'default')"#).execute(pool).await.unwrap();
        sqlx::query("INSERT INTO user_api_keys (user_id, key, status, remain_quota) VALUES ('local-test', 'sk-local-test', 1, 1000000000)").execute(pool).await.unwrap();
        seed_price(&db, model).await.unwrap();
        let price_cache = burncloud_commerce_billing::PriceCache::load(&db)
            .await
            .unwrap();
        let (log_tx, logs) = mpsc::channel(32);
        let (request_log_tx, requests) = mpsc::channel(32);
        let (force_sync_tx, _force_sync_rx) = mpsc::channel(1);
        let (budget_update_tx, _budget_rx) = mpsc::channel(1);
        let state = AppState {
            client: reqwest::Client::builder().no_proxy().build().unwrap(),
            db: db.clone(),
            jwt_secret: burncloud_service_user::JwtSecret::new("local-test-only").unwrap(),
            balancer: Arc::new(RoundRobinBalancer::new()),
            limiter: Arc::new(RateLimiter::new(100.0, 10.0)),
            circuit_breaker: Arc::new(CircuitBreaker::new(
                CIRCUIT_BREAKER_FAILURE_THRESHOLD,
                CIRCUIT_BREAKER_COOLDOWN_SECS,
            )),
            log_tx,
            request_log_tx,
            model_router: Arc::new(ModelRouter::new(db.clone())),
            route_miss_responder: Arc::new(|_| None),
            channel_state_tracker: Arc::new(ChannelStateTracker::new()),
            adaptor_factory: Arc::new(adaptor::factory::DynamicAdaptorFactory::new(db.clone())),
            api_version_detector: Arc::new(adaptor::detector::ApiVersionDetector::new(db.clone())),
            cost_calculator: burncloud_commerce_billing::CostCalculator::new(price_cache.clone()),
            price_cache,
            force_sync_tx,
            exchange_rate_service: Arc::new(exchange_rate::ExchangeRateService::new(db.clone())),
            scheduler_policies: Arc::new(tokio::sync::RwLock::new(Default::default())),
            affinity_cache: Arc::new(affinity::AffinityCache::default()),
            rate_budget: Arc::new(rate_budget::InMemoryBudget::new()),
            fail_open_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            billing_strict: true,
            billing_preflight_rejected_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            billing_post_settle_price_missing_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            budget_update_tx,
            request_log_storage_policy: StoragePolicy::Summary,
            empty_response_counter: Arc::new(EmptyResponseCounter::new()),
            channel_health_manager: Arc::new(channel_health_manager::ChannelHealthManager::new()),
        };
        Self {
            file,
            db,
            state,
            logs,
            _requests: requests,
        }
    }

    async fn attach(
        &self,
        model: &str,
        base_url: &str,
    ) -> crate::local_attachment::LocalRouteAttachmentId {
        LocalAttachmentState::for_database(&self.db)
            .attach(
                &self.db,
                LocalRouteAttachment {
                    model: model.into(),
                    base_url: base_url.into(),
                },
                Some(tokio::time::Instant::now()),
            )
            .await
            .unwrap()
    }

    async fn close(self) {
        self.db.get_connection().unwrap().pool().close().await;
        // SQLite 后台线程释放文件句柄后，再显式验证测试文件可删除。
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        self.file.close().unwrap();
    }
}

async fn chat(state: AppState, model: &str, path: &str) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        HeaderValue::from_static("Bearer sk-local-test"),
    );
    headers.insert("x-api-key", HeaderValue::from_static("sk-local-test"));
    headers.insert("x-goog-api-key", HeaderValue::from_static("sk-local-test"));
    proxy_handler(State(state), Method::POST, path.parse().unwrap(), headers,
        Body::from(serde_json::json!({"model": model, "messages": [{"role":"user", "content":"test"}], "max_tokens": 16}).to_string())).await
}

async fn upstream(app: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (address, task)
}

async fn stop(task: tokio::task::JoinHandle<()>) {
    task.abort();
    if let Err(error) = task.await {
        assert!(error.is_cancelled());
    }
}

fn response_json(model: &str) -> serde_json::Value {
    serde_json::json!({"id":"local-test-response", "object":"chat.completion", "created":1, "model":model,
        "choices":[{"index":0,"message":{"role":"assistant","content":"controlled success"},"finish_reason":"stop"}],
        "usage":{"prompt_tokens":2,"completion_tokens":3,"total_tokens":5}})
}

#[tokio::test]
async fn authenticated_dispatch_preserves_alias_path_credentials_feedback_and_billing() {
    let model = "Qwen:中文 4b";
    let mut fixture = Fixture::new(model).await;
    let (tx, mut received) = mpsc::channel(4);
    let app = Router::new().fallback(axum::routing::post(
        move |uri: Uri, headers: HeaderMap, axum::Json(body): axum::Json<serde_json::Value>| {
            let tx = tx.clone();
            async move {
                tx.send((uri, headers, body)).await.unwrap();
                axum::Json(response_json("Qwen:中文 4b"))
            }
        },
    ));
    let (url, server) = upstream(app).await;
    let id = fixture.attach(model, &url).await;
    for path in ["/v1/chat/completions", "/v1/completions"] {
        let response = chat(fixture.state.clone(), model, path).await;
        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(status, StatusCode::OK, "{body:?}");
        let (uri, headers, body) = received.recv().await.unwrap();
        assert_eq!(uri.path(), path);
        assert_eq!(body["model"], model);
        for name in ["authorization", "x-api-key", "x-goog-api-key"] {
            assert!(!headers.contains_key(name));
        }
        let log = fixture.logs.recv().await.unwrap();
        assert_eq!(log.model.as_deref(), Some(model));
        // 本票聊天路径验证实际计费；既有转换路径的 OpenAI 流式优化在 handler 返回后消费用量，
        // 这里只验证模型、路径和凭证，不把其既有结算时序当成本票已完成的能力。
        if path == "/v1/chat/completions" {
            assert_eq!(log.cost_status.as_deref(), Some("ok"), "{log:?}");
            assert!(log.cost > 0);
        }
        let states = fixture.state.channel_state_tracker.get_all_states();
        let state = &states
            .iter()
            .find(|(channel, _)| *channel == id.0)
            .unwrap()
            .1;
        assert!(state.models.contains_key(model));
        assert!(!state.models.contains_key("Qwen"));
    }
    ExistingRouterLocalAttacher::new(fixture.db.clone())
        .quarantine(id)
        .await
        .unwrap();
    assert_eq!(
        chat(fixture.state.clone(), model, "/v1/chat/completions")
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    stop(server).await;
    fixture.close().await;
}

#[tokio::test]
async fn stale_snapshot_is_rejected_after_quarantine_or_replacement_before_send() {
    for path in ["/v1/chat/completions", "/v1/completions"] {
        for detach in [false, true] {
            let fixture = Fixture::new("stale:test").await;
            let count = Arc::new(AtomicUsize::new(0));
            let hits = count.clone();
            let (url, server) = upstream(Router::new().fallback(axum::routing::post(move || {
                let hits = hits.clone();
                async move {
                    hits.fetch_add(1, Ordering::SeqCst);
                    axum::Json(response_json("stale:test"))
                }
            })))
            .await;
            let id = fixture.attach("stale:test", &url).await;
            let local = LocalAttachmentState::for_database(&fixture.db);
            let initialized = Arc::new(Notify::new());
            let release = Arc::new(Notify::new());
            let init_notify = initialized.clone();
            let init_release = release.clone();
            // 利用已有客户端初始化 await 建立白盒屏障，不增加生产测试接口。
            let init = tokio::spawn(async move {
                local
                    .client
                    .get_or_init(|| async {
                        init_notify.notify_one();
                        init_release.notified().await;
                        reqwest::Client::builder()
                            .no_proxy()
                            .redirect(reqwest::redirect::Policy::none())
                            .build()
                            .unwrap()
                    })
                    .await;
            });
            initialized.notified().await;
            let state = fixture.state.clone();
            let job = tokio::spawn(async move { chat(state, "stale:test", path).await });
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                while fixture.state.fail_open_count.load(Ordering::SeqCst) == 0 {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            let manager = ExistingRouterLocalAttacher::new(fixture.db.clone());
            if detach {
                manager.detach(id).await.unwrap();
                let new_id = fixture.attach("stale:test", &url).await;
                assert_ne!(id, new_id);
            } else {
                manager.quarantine(id).await.unwrap();
            }
            release.notify_one();
            init.await.unwrap();
            let response = tokio::time::timeout(std::time::Duration::from_secs(5), job)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(response.headers()["X-Rejected-By"], "scheduler");
            assert_eq!(response.headers()["Retry-After"], "60");
            let body: serde_json::Value =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            assert_eq!(body["error"]["type"], "service_unavailable");
            assert_eq!(body["error"]["code"], "no_available_channels");
            assert_eq!(count.load(Ordering::SeqCst), 0);
            if detach {
                assert_eq!(
                    chat(fixture.state.clone(), "stale:test", path)
                        .await
                        .status(),
                    StatusCode::OK
                );
                assert_eq!(count.load(Ordering::SeqCst), 1);
            }
            stop(server).await;
            fixture.close().await;
        }
    }
}

#[tokio::test]
async fn granted_attempt_survives_quarantine_but_retry_gets_new_admission() {
    let fixture = Fixture::new("retry:test").await;
    fixture
        .state
        .scheduler_policies
        .write()
        .await
        .insert("default".into(), scheduler::SchedulerKind::Passthrough);
    let arrived = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let (first_url, first_server) = {
        let arrived = arrived.clone();
        let release = release.clone();
        upstream(Router::new().fallback(axum::routing::post(move || {
            let arrived = arrived.clone();
            let release = release.clone();
            async move {
                arrived.notify_one();
                release.notified().await;
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({"error":"controlled failure"})),
                )
            }
        })))
        .await
    };
    let hits = Arc::new(AtomicUsize::new(0));
    let other_hits = hits.clone();
    let (second_url, second_server) =
        upstream(Router::new().fallback(axum::routing::post(move || {
            let hits = other_hits.clone();
            async move {
                hits.fetch_add(1, Ordering::SeqCst);
                axum::Json(response_json("retry:test"))
            }
        })))
        .await;
    let first_id = fixture.attach("retry:test", &first_url).await;
    let second_id = fixture.attach("retry:test", &second_url).await;
    fixture
        .state
        .affinity_cache
        .insert("local-test", "retry:test", first_id.0);
    let state = fixture.state.clone();
    let job = tokio::spawn(async move { chat(state, "retry:test", "/v1/chat/completions").await });
    tokio::time::timeout(std::time::Duration::from_secs(5), arrived.notified())
        .await
        .unwrap();
    let manager = ExistingRouterLocalAttacher::new(fixture.db.clone());
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        manager.quarantine(first_id),
    )
    .await
    .unwrap()
    .unwrap();
    manager.quarantine(second_id).await.unwrap();
    release.notify_one();
    let response = tokio::time::timeout(std::time::Duration::from_secs(5), job)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(!response.headers().contains_key("X-Rejected-By"));
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    stop(first_server).await;
    stop(second_server).await;
    fixture.close().await;
}

#[tokio::test]
async fn local_redirect_is_not_followed_and_non_local_path_keeps_legacy_behavior() {
    let fixture = Fixture::new("redirect:test").await;
    let escaped = Arc::new(AtomicUsize::new(0));
    let hits = escaped.clone();
    let (target, target_server) =
        upstream(Router::new().fallback(axum::routing::post(move || {
            let hits = hits.clone();
            async move {
                hits.fetch_add(1, Ordering::SeqCst);
                axum::Json(response_json("redirect:test"))
            }
        })))
        .await;
    let redirect_target = format!("{target}/redirected");
    let (path_tx, mut paths) = mpsc::channel(4);
    let (url, server) = upstream(Router::new().fallback(axum::routing::post(move |uri: Uri| {
        let target = redirect_target.clone();
        let path_tx = path_tx.clone();
        async move {
            path_tx.send(uri.path().to_string()).await.unwrap();
            (StatusCode::TEMPORARY_REDIRECT, [("location", target)], "")
        }
    })))
    .await;
    let id = fixture.attach("redirect:test", &url).await;
    let response = chat(
        fixture.state.clone(),
        "redirect:test",
        "/v1/chat/completions",
    )
    .await;
    assert!(!response.status().is_success());
    assert_eq!(escaped.load(Ordering::SeqCst), 0);
    assert_eq!(paths.recv().await.unwrap(), "/v1/chat/completions");
    // 仅在隔离测试库建立独立非本机供给，避免继承上一节点的故障评分；普通客户端语义不变。
    let mut channel = ChannelService::get_by_id(&fixture.db, id.0)
        .await
        .unwrap()
        .unwrap();
    channel.tag = None;
    channel.id = 0;
    ChannelService::create(&fixture.db, &mut channel)
        .await
        .unwrap();
    ExistingRouterLocalAttacher::new(fixture.db.clone())
        .detach(id)
        .await
        .unwrap();
    let response = chat(
        fixture.state.clone(),
        "redirect:test",
        "/v1/chat/completions",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(escaped.load(Ordering::SeqCst), 1);
    assert_eq!(paths.recv().await.unwrap(), "/chat/completions");
    stop(server).await;
    stop(target_server).await;
    fixture.close().await;
}

#[tokio::test]
async fn auth_billing_shaper_and_true_route_miss_keep_their_error_branches() {
    let fixture = Fixture::new("errors:test").await;
    let (url, server) = upstream(Router::new().fallback(axum::routing::post(|| async {
        axum::Json(response_json("errors:test"))
    })))
    .await;
    let id = fixture.attach("errors:test", &url).await;
    let response = proxy_handler(
        State(fixture.state.clone()),
        Method::POST,
        "/v1/chat/completions".parse().unwrap(),
        HeaderMap::new(),
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    fixture
        .state
        .rate_budget
        .configure(id.0, 0, 0, rate_budget::ChannelReservation::default());
    let response = chat(fixture.state.clone(), "errors:test", "/v1/chat/completions").await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response.headers()["X-Rejected-By"], "shaper");
    fixture.attach("no-price", &url).await;
    assert_eq!(
        chat(fixture.state.clone(), "no-price", "/v1/chat/completions")
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        chat(
            fixture.state.clone(),
            "not-registered",
            "/v1/chat/completions"
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    sqlx::query("UPDATE user_api_keys SET remain_quota = 0 WHERE key = 'sk-local-test'")
        .execute(fixture.db.get_connection().unwrap().pool())
        .await
        .unwrap();
    assert_eq!(
        chat(fixture.state.clone(), "errors:test", "/v1/chat/completions")
            .await
            .status(),
        StatusCode::PAYMENT_REQUIRED
    );
    stop(server).await;
    fixture.close().await;
}

async fn streaming_chat(state: AppState, model: &str, path: &str) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        HeaderValue::from_static("Bearer sk-local-test"),
    );
    proxy_handler(State(state), Method::POST, path.parse().unwrap(), headers,
        Body::from(serde_json::json!({"model":model,"messages":[{"role":"user","content":"test"}],"stream":true}).to_string())).await
}

#[tokio::test]
async fn streaming_holds_slot_until_eof_or_cancel_on_both_send_paths() {
    for path in ["/v1/chat/completions", "/v1/completions"] {
        let fixture = Fixture::new("stream:slots").await;
        let release = Arc::new(Notify::new());
        let signal = release.clone();
        let (url, server) = upstream(Router::new().fallback(axum::routing::post(move || {
            let signal = signal.clone();
            async move {
                let first = futures::stream::once(async { Ok::<_, std::io::Error>(axum::body::Bytes::from_static(b"data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"id\":\"a\"}]}}]}\n\n")) });
                let last = futures::stream::once(async move { signal.notified().await; Ok::<_, std::io::Error>(axum::body::Bytes::from_static(b"data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\ndata: [DONE]\n\n")) });
                Response::builder().header("content-type", "text/event-stream").body(Body::from_stream(first.chain(last))).unwrap()
            }
        }))).await;
        let id = fixture.attach("stream:slots", &url).await;
        let local = LocalAttachmentState::for_database(&fixture.db);
        let response = streaming_chat(fixture.state.clone(), "stream:slots", path).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!local.lifecycle_allows(id.0, "stream:slots", &url));
        assert!(fixture
            .state
            .channel_state_tracker
            .get_all_states()
            .is_empty());
        let mut body = response.into_body();
        assert!(body.frame().await.unwrap().is_ok());
        assert_eq!(
            chat(fixture.state.clone(), "stream:slots", path)
                .await
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        drop(body);
        assert!(local.lifecycle_allows(id.0, "stream:slots", &url));
        let response = streaming_chat(fixture.state.clone(), "stream:slots", path).await;
        let mut body = response.into_body();
        assert!(body.frame().await.unwrap().is_ok());
        release.notify_one();
        body.collect().await.unwrap();
        assert!(local.lifecycle_allows(id.0, "stream:slots", &url));
        assert!(!fixture
            .state
            .channel_state_tracker
            .get_all_states()
            .is_empty());
        stop(server).await;
        fixture.close().await;
    }
}

#[tokio::test]
async fn streaming_error_after_content_locks_attachment_and_releases_slot() {
    let fixture = Fixture::new("stream:fault").await;
    let (url, server) = upstream(Router::new().fallback(axum::routing::post(|| async {
        let chunks = [
            b"data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\ndata: {\"err".as_slice(),
            b"or\":{\"code\":\"server_error\"}}\n\n".as_slice(),
        ];
        Body::from_stream(futures::stream::iter(
            chunks
                .into_iter()
                .map(|c| Ok::<_, std::io::Error>(axum::body::Bytes::copy_from_slice(c))),
        ))
    })))
    .await;
    let id = fixture.attach("stream:fault", &url).await;
    let response = streaming_chat(
        fixture.state.clone(),
        "stream:fault",
        "/v1/chat/completions",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.into_body().collect().await.is_err());
    let local = LocalAttachmentState::for_database(&fixture.db);
    {
        let index = local.index.lock().unwrap();
        let slot = &index.targets[&index.ids[&id.0]];
        assert!(slot.faulted);
        assert_eq!(slot.in_flight.load(Ordering::Acquire), 0);
    }
    assert_eq!(
        chat(
            fixture.state.clone(),
            "stream:fault",
            "/v1/chat/completions"
        )
        .await
        .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    stop(server).await;
    fixture.close().await;
}

#[tokio::test]
async fn full_candidates_do_not_hide_sixth_available_attachment() {
    let fixture = Fixture::new("six:slots").await;
    let local = LocalAttachmentState::for_database(&fixture.db);
    let mut servers = Vec::new();
    let mut permits = Vec::new();
    for n in 0..6 {
        let (url, server) = upstream(Router::new().fallback(axum::routing::post(|| async {
            axum::Json(response_json("six:slots"))
        })))
        .await;
        let id = fixture.attach("six:slots", &url).await;
        if n < 5 {
            permits.push(local.try_acquire(id.0, "six:slots", &url).unwrap());
        }
        servers.push(server);
    }
    assert_eq!(
        chat(fixture.state.clone(), "six:slots", "/v1/chat/completions")
            .await
            .status(),
        StatusCode::OK
    );
    drop(permits);
    for server in servers {
        stop(server).await;
    }
    fixture.close().await;
}

#[tokio::test]
async fn nonstream_request_errors_do_not_poison_health_but_server_errors_lock() {
    let fixture = Fixture::new("http:errors").await;
    let status = Arc::new(std::sync::atomic::AtomicU16::new(400));
    let flag = status.clone();
    let hits = Arc::new(AtomicUsize::new(0));
    let count = hits.clone();
    let (url, server) = upstream(Router::new().fallback(axum::routing::post(move || {
        let status = flag.load(Ordering::SeqCst);
        count.fetch_add(1, Ordering::SeqCst);
        async move {
            (
                StatusCode::from_u16(status).unwrap(),
                axum::Json(serde_json::json!({"error":{"code":"invalid_request"}})),
            )
        }
    })))
    .await;
    let id = fixture.attach("http:errors", &url).await;
    for _ in 0..2 {
        assert_eq!(
            chat(fixture.state.clone(), "http:errors", "/v1/chat/completions")
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert!(fixture
        .state
        .channel_state_tracker
        .get_all_states()
        .is_empty());
    status.store(500, Ordering::SeqCst);
    assert_eq!(
        chat(fixture.state.clone(), "http:errors", "/v1/chat/completions")
            .await
            .status(),
        StatusCode::BAD_GATEWAY
    );
    assert_eq!(
        chat(fixture.state.clone(), "http:errors", "/v1/chat/completions")
            .await
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(hits.load(Ordering::SeqCst), 3);
    assert!(
        !LocalAttachmentState::for_database(&fixture.db).lifecycle_allows(
            id.0,
            "http:errors",
            &url
        )
    );
    stop(server).await;
    fixture.close().await;
}

#[tokio::test]
async fn price_policy_preserves_highest_tier_and_scheduler_error_contract() {
    let fixture = Fixture::new("price:strict").await;
    sqlx::query("UPDATE billing_prices SET currency = 'CNY', region = 'foreign' WHERE model = ?")
        .bind("price:strict")
        .execute(fixture.db.get_connection().unwrap().pool())
        .await
        .unwrap();
    seed_price(&fixture.db, "price:strict").await.unwrap();
    fixture
        .state
        .price_cache
        .refresh(&fixture.db)
        .await
        .unwrap();
    fixture
        .state
        .scheduler_policies
        .write()
        .await
        .insert("default".into(), scheduler::SchedulerKind::PriceFirst);
    let hits = Arc::new(AtomicUsize::new(0));
    let mut servers = Vec::new();
    for priority in [10, 0] {
        let count = hits.clone();
        let (url, server) = upstream(Router::new().fallback(axum::routing::post(move || {
            count.fetch_add(1, Ordering::SeqCst);
            async { axum::Json(response_json("price:strict")) }
        })))
        .await;
        let id = fixture.attach("price:strict", &url).await;
        let mut channel = ChannelService::get_by_id(&fixture.db, id.0)
            .await
            .unwrap()
            .unwrap();
        channel.priority = priority;
        if priority == 10 {
            channel.pricing_region = Some("foreign".into());
        }
        ChannelService::update(&fixture.db, &channel).await.unwrap();
        if priority == 0 {
            fixture
                .state
                .affinity_cache
                .insert("local-test", "price:strict", id.0);
        }
        servers.push(server);
    }
    let response = chat(
        fixture.state.clone(),
        "price:strict",
        "/v1/chat/completions",
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response.headers()["X-Rejected-By"], "scheduler");
    assert_eq!(response.headers()["Retry-After"], "60");
    let body: serde_json::Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["error"]["code"], "no_comparable_price");
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    for server in servers {
        stop(server).await;
    }
    fixture.close().await;
}

#[tokio::test]
async fn strict_policy_limits_actual_sends_to_five_without_reusing_permits() {
    let fixture = Fixture::new("retry:five").await;
    let hits = Arc::new(AtomicUsize::new(0));
    let mut servers = Vec::new();
    for _ in 0..7 {
        let count = hits.clone();
        let (url, server) = upstream(Router::new().fallback(axum::routing::post(move || {
            count.fetch_add(1, Ordering::SeqCst);
            async { StatusCode::INTERNAL_SERVER_ERROR }
        })))
        .await;
        fixture.attach("retry:five", &url).await;
        servers.push(server);
    }
    assert_eq!(
        chat(fixture.state.clone(), "retry:five", "/v1/chat/completions")
            .await
            .status(),
        StatusCode::BAD_GATEWAY
    );
    assert_eq!(hits.load(Ordering::SeqCst), 5);
    for server in servers {
        stop(server).await;
    }
    fixture.close().await;
}
