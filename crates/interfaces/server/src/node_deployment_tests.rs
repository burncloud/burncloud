#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "部署事件桥接测试使用临时数据库和本地测试服务，断言前提失败应直接定位。"
)]

use super::*;
use burncloud_database::create_database_with_url;
use burncloud_node_runtime::{
    deployment::{DeploymentCapacity, DeploymentEvent, DeploymentEventKind, DeploymentStage},
    NodeState,
};
use burncloud_router::{
    local_attachment::{ExistingRouterLocalAttacher, LocalRouteAttacher},
    model_router::ModelRouter,
    RouterDatabase,
};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn database() -> (tempfile::NamedTempFile, Arc<burncloud_database::Database>) {
    let file = tempfile::NamedTempFile::new().unwrap();
    let url = burncloud_database::sqlite_url(&file.path().to_string_lossy(), cfg!(windows), true);
    let db = Arc::new(create_database_with_url(&url).await.unwrap());
    RouterDatabase::init(&db).await.unwrap();
    (file, db)
}

async fn model_probe_server(model: &str) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let model = model.to_string();
    let task = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            let mut request = [0_u8; 1024];
            if stream.read(&mut request).await.is_err() {
                continue;
            }
            let body = format!(r#"{{"data":[{{"id":"{model}"}}]}}"#);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            );
            if stream.write_all(response.as_bytes()).await.is_err() {
                continue;
            }
        }
    });
    (format!("http://{address}"), task)
}

fn event(
    model: &str,
    endpoint: &str,
    generation: u64,
    kind: DeploymentEventKind,
    capacity: Option<DeploymentCapacity>,
) -> DeploymentEvent {
    DeploymentEvent {
        model_id: model.to_string(),
        kind,
        state: Some(NodeState::Ready),
        error_kind: None,
        local_endpoint: Some(endpoint.to_string()),
        pid: Some(7),
        error: None,
        cleanup_error: None,
        generation,
        capacity,
    }
}

fn capacity(free_slots: u32) -> DeploymentCapacity {
    DeploymentCapacity {
        total_slots: 1,
        free_slots,
        observed_at_unix_ms: 1,
    }
}

#[tokio::test]
async fn ready_event_automatically_registers_and_lifecycle_events_remove_route() {
    let (_file, db) = database().await;
    let model = "qwen-4b";
    let (endpoint, server) = model_probe_server(model).await;
    let attacher = Arc::new(ExistingRouterLocalAttacher::new(db.clone()));
    let events = DeploymentEventStore::with_attacher(attacher.clone());
    let router = ModelRouter::new(db.clone());

    events
        .publish(event(
            model,
            &endpoint,
            42,
            DeploymentEventKind::InitialResult(NodeState::Ready),
            Some(capacity(1)),
        ))
        .await
        .unwrap();
    let snapshot = events.get(model).await.unwrap();
    assert_eq!(snapshot.route_status, "routable");
    let attachment_id = LocalRouteAttachmentId(snapshot.route_attachment_id.unwrap());
    assert!(attachment_id.0 > 0);
    assert_eq!(
        router.get_candidates("default", model).await.unwrap().len(),
        1
    );

    events
        .publish(event(
            model,
            &endpoint,
            42,
            DeploymentEventKind::Heartbeat(NodeState::Ready),
            Some(capacity(0)),
        ))
        .await
        .unwrap();
    let snapshot = events.get(model).await.unwrap();
    assert_eq!(snapshot.route_status, "saturated");
    // 满载只从派发准入中排除，模型身份仍保持可见。
    assert_eq!(
        router.get_candidates("default", model).await.unwrap().len(),
        1
    );

    events
        .publish(event(
            model,
            &endpoint,
            42,
            DeploymentEventKind::StateChanged(NodeState::Unhealthy),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(events.get(model).await.unwrap().route_status, "quarantined");
    assert!(router
        .get_candidates("default", model)
        .await
        .unwrap()
        .is_empty());

    events
        .publish(DeploymentEvent {
            kind: DeploymentEventKind::Stopped,
            state: Some(NodeState::Absent),
            ..event(model, &endpoint, 42, DeploymentEventKind::Stopped, None)
        })
        .await
        .unwrap();
    let snapshot = events.get(model).await.unwrap();
    assert_eq!(snapshot.route_status, "detached");
    assert!(snapshot.route_attachment_id.is_none());
    assert!(router
        .get_candidates("default", model)
        .await
        .unwrap()
        .is_empty());

    attacher.detach(attachment_id).await.unwrap();
    server.abort();
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn heartbeat_from_stopped_generation_cannot_reactivate_the_route() {
    let (_file, db) = database().await;
    let model = "stopped-generation";
    let (endpoint, server) = model_probe_server(model).await;
    let attacher = Arc::new(ExistingRouterLocalAttacher::new(db.clone()));
    let events = DeploymentEventStore::with_attacher(attacher);
    let router = ModelRouter::new(db.clone());

    events
        .publish(event(
            model,
            &endpoint,
            4,
            DeploymentEventKind::InitialResult(NodeState::Ready),
            Some(capacity(1)),
        ))
        .await
        .unwrap();
    events
        .publish(DeploymentEvent {
            state: Some(NodeState::Absent),
            ..event(model, &endpoint, 4, DeploymentEventKind::Stopped, None)
        })
        .await
        .unwrap();
    events
        .publish(event(
            model,
            &endpoint,
            4,
            DeploymentEventKind::Heartbeat(NodeState::Ready),
            Some(capacity(1)),
        ))
        .await
        .unwrap();

    let snapshot = events.get(model).await.unwrap();
    assert_eq!(snapshot.status, "stopped");
    assert_eq!(snapshot.route_status, "detached");
    assert!(snapshot.route_attachment_id.is_none());
    assert!(router
        .get_candidates("default", model)
        .await
        .unwrap()
        .is_empty());

    server.abort();
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn failed_state_event_quarantines_and_detaches_the_local_route() {
    let (_file, db) = database().await;
    let model = "failed-state";
    let (endpoint, server) = model_probe_server(model).await;
    let attacher = Arc::new(ExistingRouterLocalAttacher::new(db.clone()));
    let events = DeploymentEventStore::with_attacher(attacher);
    let router = ModelRouter::new(db.clone());

    events
        .publish(event(
            model,
            &endpoint,
            3,
            DeploymentEventKind::InitialResult(NodeState::Ready),
            Some(capacity(1)),
        ))
        .await
        .unwrap();
    assert_eq!(
        router.get_candidates("default", model).await.unwrap().len(),
        1
    );

    let mut failed = event(
        model,
        &endpoint,
        3,
        DeploymentEventKind::StateChanged(NodeState::Failed),
        None,
    );
    failed.state = Some(NodeState::Failed);
    events.publish(failed).await.unwrap();

    let snapshot = events.get(model).await.unwrap();
    assert_eq!(snapshot.status, "failed");
    assert_eq!(snapshot.state.as_deref(), Some("failed"));
    assert_eq!(snapshot.route_status, "failed");
    assert!(snapshot.route_attachment_id.is_none());
    assert!(router
        .get_candidates("default", model)
        .await
        .unwrap()
        .is_empty());

    server.abort();
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn missing_initial_capacity_stays_unroutable_until_a_valid_heartbeat() {
    let (_file, db) = database().await;
    let model = "capacity-late";
    let (endpoint, server) = model_probe_server(model).await;
    let events =
        DeploymentEventStore::with_attacher(Arc::new(ExistingRouterLocalAttacher::new(db.clone())));

    events
        .publish(event(
            model,
            &endpoint,
            9,
            DeploymentEventKind::InitialResult(NodeState::Ready),
            None,
        ))
        .await
        .unwrap();
    let snapshot = events.get(model).await.unwrap();
    assert_eq!(snapshot.route_status, "unroutable");
    assert!(snapshot.route_attachment_id.is_none());

    events
        .publish(event(
            model,
            &endpoint,
            9,
            DeploymentEventKind::Heartbeat(NodeState::Ready),
            Some(capacity(1)),
        ))
        .await
        .unwrap();
    let snapshot = events.get(model).await.unwrap();
    assert_eq!(snapshot.route_status, "routable");
    assert!(snapshot.route_attachment_id.is_some());

    server.abort();
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn stale_generation_event_cannot_replace_current_route() {
    let (_file, db) = database().await;
    let model = "generation-guard";
    let (endpoint, server) = model_probe_server(model).await;
    let attacher = Arc::new(ExistingRouterLocalAttacher::new(db.clone()));
    let events = DeploymentEventStore::with_attacher(attacher);

    events
        .publish(event(
            model,
            &endpoint,
            2,
            DeploymentEventKind::InitialResult(NodeState::Ready),
            Some(capacity(1)),
        ))
        .await
        .unwrap();
    let current = events.get(model).await.unwrap();
    assert_eq!(current.generation, 2);
    let current_id = current.route_attachment_id;

    events
        .publish(event(
            model,
            &endpoint,
            1,
            DeploymentEventKind::Heartbeat(NodeState::Ready),
            Some(capacity(1)),
        ))
        .await
        .unwrap();
    let snapshot = events.get(model).await.unwrap();
    assert_eq!(snapshot.generation, 2);
    assert_eq!(snapshot.route_attachment_id, current_id);

    server.abort();
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn redeployment_after_failed_generation_accepts_new_endpoint_events() {
    let (_file, db) = database().await;
    let model = "redeploy-after-failure";
    let (old_endpoint, old_server) = model_probe_server(model).await;
    let (new_endpoint, new_server) = model_probe_server(model).await;
    let events =
        DeploymentEventStore::with_attacher(Arc::new(ExistingRouterLocalAttacher::new(db.clone())));

    events
        .publish(event(
            model,
            &old_endpoint,
            1,
            DeploymentEventKind::InitialResult(NodeState::Ready),
            Some(capacity(1)),
        ))
        .await
        .unwrap();
    let failed = DeploymentEvent {
        state: Some(NodeState::Failed),
        ..event(
            model,
            &old_endpoint,
            1,
            DeploymentEventKind::StateChanged(NodeState::Failed),
            None,
        )
    };
    events.publish(failed).await.unwrap();

    events.insert_queued(model).await;
    events
        .publish(DeploymentEvent {
            model_id: model.to_string(),
            kind: DeploymentEventKind::Progress(DeploymentStage::Planning),
            state: None,
            error_kind: None,
            local_endpoint: None,
            pid: None,
            error: None,
            cleanup_error: None,
            generation: 2,
            capacity: None,
        })
        .await
        .unwrap();
    events
        .publish(event(
            model,
            &new_endpoint,
            2,
            DeploymentEventKind::Progress(DeploymentStage::Starting),
            None,
        ))
        .await
        .unwrap();
    events
        .publish(event(
            model,
            &new_endpoint,
            2,
            DeploymentEventKind::InitialResult(NodeState::Ready),
            Some(capacity(1)),
        ))
        .await
        .unwrap();

    let snapshot = events.get(model).await.unwrap();
    assert_eq!(snapshot.generation, 2);
    assert_eq!(snapshot.endpoint.as_deref(), Some(new_endpoint.as_str()));
    assert_eq!(snapshot.route_status, "routable");
    assert!(snapshot.route_attachment_id.is_some());

    old_server.abort();
    new_server.abort();
    db.get_connection().unwrap().pool().close().await;
}
