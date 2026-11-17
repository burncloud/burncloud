#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "测试固定输入与断言失败应中止"
)]
use super::*;
use crate::local_attachment::{ExistingRouterLocalAttacher, LocalRouteAttacher};
use axum::{body::Body, http::StatusCode, response::Response, routing::get, Json, Router};
use burncloud_supply_channel::ChannelService;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::task::JoinHandle;

async fn database() -> (tempfile::NamedTempFile, Arc<Database>) {
    let file = tempfile::NamedTempFile::new().unwrap();
    let url = burncloud_database::sqlite_url(&file.path().to_string_lossy(), cfg!(windows), true);
    let db = Arc::new(
        burncloud_database::create_database_with_url(&url)
            .await
            .unwrap(),
    );
    (file, db)
}

struct ModelServer {
    url: String,
    task: JoinHandle<()>,
}

impl Drop for ModelServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn model_server(app: Router) -> ModelServer {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    ModelServer { url, task }
}

fn attachment(url: &str) -> LocalRouteAttachment {
    LocalRouteAttachment {
        model: "Qwen:4b".into(),
        base_url: url.into(),
    }
}

fn model_list() -> serde_json::Value {
    serde_json::json!({"data":[{"id":"Qwen:4b"},{"id":"other"}]})
}

async fn wait_for_channels(
    db: &Database,
    count: usize,
) -> Vec<burncloud_supply_contracts::Channel> {
    timeout(Duration::from_secs(8), async {
        loop {
            let channels = ChannelService::list(db, 100, 0).await.unwrap();
            if channels.len() == count {
                return channels;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}

async fn cancel(task: JoinHandle<Result<(), LocalRouteAttachmentError>>) {
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
}

#[tokio::test]
async fn constructor_validates_whole_list_before_claiming_session() {
    let (_file, db) = database().await;
    let valid = attachment("http://127.0.0.1");
    for invalid in [
        LocalRouteAttachment {
            model: " invalid".into(),
            ..valid.clone()
        },
        attachment("http://localhost"),
        attachment("http://127.0.0.1/v1"),
        attachment("http://127.0.0.1/../"),
    ] {
        assert!(LocalRouteManager::new(db.clone(), vec![valid.clone(), invalid]).is_err());
    }
    assert!(LocalRouteManager::new(
        db.clone(),
        vec![valid.clone(), attachment("http://127.0.0.1:80/")]
    )
    .is_err());
    // 无效清单不能认领会话或改变既有直接 attach 的行为。
    let id = LocalAttachmentState::for_database(&db)
        .attach(&db, valid.clone(), Some(Instant::now()))
        .await
        .unwrap();
    let local = LocalAttachmentState::for_database(&db);
    assert!(local.lifecycle_allows(id.0, &valid.model, &valid.base_url));
    db.get_connection().unwrap().pool().close().await;
    // 已关闭连接池仍可构造，证明构造路径不依赖 SQL 或网络可达。
    let manager = LocalRouteManager::new(db.clone(), vec![valid.clone()]).unwrap();
    assert!(!local.lifecycle_allows(id.0, &valid.model, &valid.base_url));
    assert!(LocalRouteManager::new(db.clone(), vec![]).is_err());
    drop(manager);
    drop(LocalRouteManager::new(db, vec![]).unwrap());
}

#[tokio::test]
async fn empty_manager_lives_until_cancelled_and_closes_new_permits() {
    let (_file, db) = database().await;
    let manager = LocalRouteManager::new(db.clone(), vec![]).unwrap();
    let mut running = Box::pin(manager.run());
    assert!(futures::poll!(running.as_mut()).is_pending());
    let target = attachment("http://127.0.0.1:18080");
    let id = LocalAttachmentState::for_database(&db)
        .attach(&db, target.clone(), Some(Instant::now()))
        .await
        .unwrap();
    let local = LocalAttachmentState::for_database(&db);
    assert!(local.lifecycle_allows(id.0, &target.model, &target.base_url));
    drop(running);
    assert!(!local.lifecycle_allows(id.0, &target.model, &target.base_url));
    assert!(ExistingRouterLocalAttacher::new(db.clone())
        .attach(attachment("http://127.0.0.1:18081"))
        .await
        .is_err());
    let mut restarted = Box::pin(LocalRouteManager::new(db.clone(), vec![]).unwrap().run());
    assert!(futures::poll!(restarted.as_mut()).is_pending());
    assert!(!local.lifecycle_allows(id.0, &target.model, &target.base_url));
    drop(restarted);
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn startup_publishes_only_configured_alias_and_restart_does_not_adopt_it() {
    let (_file, db) = database().await;
    let hits = Arc::new(AtomicUsize::new(0));
    let count = hits.clone();
    let server = model_server(Router::new().route(
        "/v1/models",
        get(move || {
            count.fetch_add(1, Ordering::SeqCst);
            async { Json(model_list()) }
        }),
    ))
    .await;
    let target = attachment(&server.url);
    let task = tokio::spawn(
        LocalRouteManager::new(db.clone(), vec![target.clone()])
            .unwrap()
            .run(),
    );
    let channels = wait_for_channels(&db, 1).await;
    let id = channels[0].id;
    let local = LocalAttachmentState::for_database(&db);
    timeout(Duration::from_secs(2), async {
        while !local.lifecycle_allows(id, &target.model, &target.base_url) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(channels[0].models, "Qwen:4b");
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    cancel(task).await;
    assert!(!local.lifecycle_allows(id, &target.model, &target.base_url));
    let manager = LocalRouteManager::new(db.clone(), vec![target.clone()]).unwrap();
    manager.local.activate_manager(manager.epoch).unwrap();
    timeout(Duration::from_secs(3), manager.run_startup(&target))
        .await
        .unwrap();
    assert_eq!(wait_for_channels(&db, 1).await[0].id, id);
    assert!(!local.lifecycle_allows(id, &target.model, &target.base_url));
    let attacher = ExistingRouterLocalAttacher::new(db.clone());
    attacher
        .detach(super::super::LocalRouteAttachmentId(id))
        .await
        .unwrap();
    assert!(ChannelService::list(&db, 100, 0).await.unwrap().is_empty());
    let replacement = attacher.attach(target.clone()).await.unwrap();
    assert_ne!(replacement.0, id);
    assert!(local.lifecycle_allows(replacement.0, &target.model, &target.base_url));
    drop(manager);
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn first_probe_retry_is_independent_and_recovers_on_five_second_tick() {
    let (_file, db) = database().await;
    let ready = Arc::new(AtomicBool::new(false));
    let hits = Arc::new(AtomicUsize::new(0));
    let state = ready.clone();
    let count = hits.clone();
    let waiting = model_server(Router::new().route(
        "/v1/models",
        get(move || {
            count.fetch_add(1, Ordering::SeqCst);
            let ready = state.load(Ordering::SeqCst);
            async move {
                (
                    if ready {
                        StatusCode::OK
                    } else {
                        StatusCode::SERVICE_UNAVAILABLE
                    },
                    Json(model_list()),
                )
            }
        }),
    ))
    .await;
    let good =
        model_server(Router::new().route("/v1/models", get(|| async { Json(model_list()) }))).await;
    let task = tokio::spawn(
        LocalRouteManager::new(
            db.clone(),
            vec![attachment(&waiting.url), attachment(&good.url)],
        )
        .unwrap()
        .run(),
    );
    let channels = wait_for_channels(&db, 1).await;
    assert_eq!(channels[0].base_url.as_deref(), Some(good.url.as_str()));
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    ready.store(true, Ordering::SeqCst);
    wait_for_channels(&db, 2).await;
    assert_eq!(hits.load(Ordering::SeqCst), 2);
    cancel(task).await;
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn probe_rejects_http_alias_shape_and_body_limit_without_publishing() {
    let (_file, db) = database().await;
    let manager = LocalRouteManager::new(db.clone(), vec![]).unwrap();
    for (status, body) in [
        (302, model_list().to_string()),
        (500, model_list().to_string()),
        (200, r#"{"data":[{"id":"qwen:4b"}]}"#.into()),
        (200, r#"{"data":[{"id":42}]}"#.into()),
        (200, r#"{"models":["Qwen:4b"]}"#.into()),
        (200, "not json".into()),
        (200, " ".repeat(MAX_MODEL_LIST_BYTES + 1)),
    ] {
        let server = model_server(Router::new().route(
            "/v1/models",
            get(move || {
                let body = body.clone();
                async move {
                    Response::builder()
                        .status(status)
                        .body(Body::from(body))
                        .unwrap()
                }
            }),
        ))
        .await;
        assert!(probe(&manager.local, "Qwen:4b", &server.url).await.is_err());
    }
    assert!(ChannelService::list(&db, 100, 0).await.unwrap().is_empty());
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn probe_does_not_follow_redirects_or_forward_authorization() {
    let (_file, db) = database().await;
    let hits = Arc::new(AtomicUsize::new(0));
    let count = hits.clone();
    let sink = model_server(Router::new().route(
        "/v1/models",
        get(move || {
            count.fetch_add(1, Ordering::SeqCst);
            async { Json(model_list()) }
        }),
    ))
    .await;
    let location = format!("{}/v1/models", sink.url);
    let redirect = model_server(Router::new().route(
        "/v1/models",
        get(move |headers: axum::http::HeaderMap| {
            assert!(!headers.contains_key("authorization"));
            let location = location.clone();
            async move {
                Response::builder()
                    .status(302)
                    .header("location", location)
                    .body(Body::empty())
                    .unwrap()
            }
        }),
    ))
    .await;
    let manager = LocalRouteManager::new(db.clone(), vec![]).unwrap();
    assert!(probe(&manager.local, "Qwen:4b", &redirect.url)
        .await
        .is_err());
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn probe_limits_streamed_body_even_without_content_length() {
    let (_file, db) = database().await;
    let server = model_server(Router::new().route(
        "/v1/models",
        get(|| async {
            let chunks =
                futures::stream::iter((0..17).map(|_| Ok::<_, std::io::Error>(vec![b' '; 65536])));
            Body::from_stream(chunks)
        }),
    ))
    .await;
    let manager = LocalRouteManager::new(db.clone(), vec![]).unwrap();
    assert!(probe(&manager.local, "Qwen:4b", &server.url)
        .await
        .unwrap_err()
        .contains("1 MiB"));
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn probe_timeout_includes_reading_slow_body() {
    let (_file, db) = database().await;
    let server = model_server(Router::new().route(
        "/v1/models",
        get(|| async {
            Body::from_stream(futures::stream::pending::<Result<Vec<u8>, std::io::Error>>())
        }),
    ))
    .await;
    let manager = LocalRouteManager::new(db.clone(), vec![]).unwrap();
    let started = tokio::time::Instant::now();
    let error = timeout(
        Duration::from_secs(4),
        probe(&manager.local, "Qwen:4b", &server.url),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert!(error.contains("2 seconds"));
    assert!(started.elapsed() >= PROBE_TIMEOUT);
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn attach_failure_stops_startup_instead_of_retrying_persistence() {
    let (_file, db) = database().await;
    let server =
        model_server(Router::new().route("/v1/models", get(|| async { Json(model_list()) }))).await;
    let target = attachment(&server.url);
    let manager = LocalRouteManager::new(db.clone(), vec![target.clone()]).unwrap();
    manager.local.activate_manager(manager.epoch).unwrap();
    let pool = db.get_connection().unwrap().pool();
    sqlx::raw_sql("CREATE TRIGGER fail_startup BEFORE INSERT ON channel_providers BEGIN SELECT RAISE(FAIL, 'startup failure'); END;").execute(pool).await.unwrap();
    timeout(Duration::from_secs(3), manager.run_startup(&target))
        .await
        .unwrap();
    sqlx::raw_sql("DROP TRIGGER fail_startup;")
        .execute(pool)
        .await
        .unwrap();
    assert!(ChannelService::list(&db, 100, 0).await.unwrap().is_empty());
    assert!(ExistingRouterLocalAttacher::new(db.clone())
        .attach(target)
        .await
        .is_err());
    drop(manager);
    pool.close().await;
}

#[tokio::test]
async fn cancelling_a_waiting_probe_cannot_publish_late_response() {
    let (_file, db) = database().await;
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let start = started.clone();
    let gate = release.clone();
    let server = model_server(Router::new().route(
        "/v1/models",
        get(move || {
            let start = start.clone();
            let gate = gate.clone();
            async move {
                start.notify_one();
                gate.notified().await;
                Json(model_list())
            }
        }),
    ))
    .await;
    let task = tokio::spawn(
        LocalRouteManager::new(db.clone(), vec![attachment(&server.url)])
            .unwrap()
            .run(),
    );
    timeout(Duration::from_secs(2), started.notified())
        .await
        .unwrap();
    cancel(task).await;
    release.notify_one();
    tokio::task::yield_now().await;
    assert!(ChannelService::list(&db, 100, 0).await.unwrap().is_empty());
    drop(LocalRouteManager::new(db.clone(), vec![]).unwrap());
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn runtime_probe_failure_stops_dispatch_and_two_real_successes_restore_it() {
    let (_file, db) = database().await;
    let ready = Arc::new(AtomicBool::new(true));
    let probes = Arc::new(AtomicUsize::new(0));
    let flag = ready.clone();
    let count = probes.clone();
    let server = model_server(Router::new().route(
        "/v1/models",
        get(move || {
            let ok = flag.load(Ordering::SeqCst);
            count.fetch_add(1, Ordering::SeqCst);
            async move {
                (
                    if ok {
                        StatusCode::OK
                    } else {
                        StatusCode::SERVICE_UNAVAILABLE
                    },
                    Json(model_list()),
                )
            }
        }),
    ))
    .await;
    let target = attachment(&server.url);
    let task = tokio::spawn(
        LocalRouteManager::new(db.clone(), vec![target.clone()])
            .unwrap()
            .run(),
    );
    let id = wait_for_channels(&db, 1).await[0].id;
    let local = LocalAttachmentState::for_database(&db);
    timeout(Duration::from_secs(3), async {
        while !local.lifecycle_allows(id, &target.model, &target.base_url) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    ready.store(false, Ordering::SeqCst);
    timeout(Duration::from_secs(7), async {
        while local.lifecycle_allows(id, &target.model, &target.base_url) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let failed = probes.load(Ordering::SeqCst);
    ready.store(true, Ordering::SeqCst);
    timeout(Duration::from_secs(7), async {
        while probes.load(Ordering::SeqCst) <= failed {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!local.lifecycle_allows(id, &target.model, &target.base_url));
    timeout(Duration::from_secs(7), async {
        while !local.lifecycle_allows(id, &target.model, &target.base_url) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(wait_for_channels(&db, 1).await[0].id, id);
    cancel(task).await;
    db.get_connection().unwrap().pool().close().await;
}
