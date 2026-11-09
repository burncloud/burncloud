#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "测试断言失败时需要立即暴露场景错误。"
)]

#[path = "support/deployment.rs"]
#[allow(dead_code, reason = "集成部署场景只使用支持模块中的部分测试替身。")]
mod support;

use std::{
    net::TcpListener,
    path::PathBuf,
    process,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use burncloud_node_runtime::{
    deployment::{
        DeploymentEventKind, DeploymentService, DeploymentStage, HttpProbe, LocalProcessManager,
    },
    NodeState, ProcessManager, ReadinessTarget,
};
use support::{
    deploy, new_service, plan, FixedAdapter, RecordingPublisher, ScriptedHealthProbe,
    ScriptedProcessManager, ScriptedReadinessProbe,
};
use tokio::time::timeout;

const PYTHON_SERVER: &str = r#"
import http.server
import sys

class Handler(http.server.BaseHTTPRequestHandler):
    count = 0

    def do_GET(self):
        Handler.count += 1
        with open(sys.argv[2], "a", encoding="utf-8") as log:
            log.write(self.path + "\n")
            log.flush()
        statuses = [int(value) for value in sys.argv[3].split(",")]
        status = statuses[min(Handler.count - 1, len(statuses) - 1)]
        self.send_response(status)
        self.send_header("Content-Length", "2")
        self.end_headers()
        self.wfile.write(b"ok")

    def log_message(self, format, *args):
        pass

http.server.HTTPServer(("127.0.0.1", int(sys.argv[1])), Handler).serve_forever()
"#;

static NEXT_LOG_ID: AtomicUsize = AtomicUsize::new(0);

fn python_plan(statuses: &[u16]) -> (burncloud_node_runtime::ProcessPlan, PathBuf) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let log_path = std::env::temp_dir().join(format!(
        "burncloud-node-deployment-{}-{}.log",
        process::id(),
        NEXT_LOG_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let status_list = statuses
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let endpoint = format!("http://127.0.0.1:{port}/health");
    (
        burncloud_node_runtime::ProcessPlan {
            process: burncloud_node_runtime::ProcessSpec {
                program: "python3".into(),
                args: vec![
                    "-u".into(),
                    "-c".into(),
                    PYTHON_SERVER.into(),
                    port.to_string(),
                    log_path.display().to_string(),
                    status_list,
                ],
            },
            readiness: ReadinessTarget { endpoint },
            local_endpoint: format!("http://127.0.0.1:{port}"),
        },
        log_path,
    )
}

fn live_service() -> (
    Arc<LocalProcessManager>,
    DeploymentService,
    Arc<RecordingPublisher>,
) {
    let processes = Arc::new(LocalProcessManager::default());
    let probe = Arc::new(HttpProbe::default());
    let publisher = Arc::new(RecordingPublisher::default());
    let (service, _errors) =
        DeploymentService::new(processes.clone(), probe.clone(), probe, publisher.clone());
    (processes, service, publisher)
}

fn remove_log(path: &PathBuf) {
    if path.exists() {
        std::fs::remove_file(path).unwrap();
    }
}

#[tokio::test]
async fn i006_i007_real_deploy_preserves_adapter_plan_and_publishes_result() {
    let (processes, service, publisher) = live_service();
    let (expected_plan, log_path) = python_plan(&[200]);
    let adapter = Arc::new(FixedAdapter::new(expected_plan.clone()));
    let outcome = deploy(&service, "model-i006", adapter.clone())
        .await
        .unwrap();
    let alive = processes.is_running(outcome.process).await.unwrap();
    let events = publisher.received.lock().await.clone();
    let actual_plan = outcome.plan.clone();
    let ready_event = events
        .iter()
        .find(|event| event.kind == DeploymentEventKind::InitialResult(NodeState::Ready));
    let expected_endpoint = expected_plan.local_endpoint.clone();
    processes.stop(outcome.process).await.unwrap();
    let stopped = processes.is_running(outcome.process).await.unwrap();
    remove_log(&log_path);

    assert_eq!(outcome.state, NodeState::Ready);
    assert_ne!(outcome.process.pid, 0);
    assert!(alive);
    assert_eq!(actual_plan, expected_plan);
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
    assert!(events.first().is_some_and(|event| {
        event.kind == DeploymentEventKind::Progress(DeploymentStage::Queued)
    }));
    assert_eq!(
        ready_event.and_then(|event| event.local_endpoint.as_deref()),
        Some(expected_endpoint.as_str())
    );
    assert_eq!(
        ready_event.and_then(|event| event.pid),
        Some(outcome.process.pid)
    );
    assert!(!stopped);
}

#[tokio::test]
async fn i016_first_health_non_2xx_returns_unhealthy_without_stopping_process() {
    let (processes, service, publisher) = live_service();
    let (process_plan, log_path) = python_plan(&[200, 503]);
    let outcome = deploy(
        &service,
        "model-i016",
        Arc::new(FixedAdapter::new(process_plan.clone())),
    )
    .await
    .unwrap();
    let alive = processes.is_running(outcome.process).await.unwrap();
    let events = publisher.received.lock().await.clone();
    let result_event = events
        .iter()
        .find(|event| event.kind == DeploymentEventKind::InitialResult(NodeState::Unhealthy));
    processes.stop(outcome.process).await.unwrap();
    let stopped = processes.is_running(outcome.process).await.unwrap();
    remove_log(&log_path);

    assert_eq!(outcome.state, NodeState::Unhealthy);
    assert!(alive);
    assert_eq!(outcome.plan.local_endpoint, process_plan.local_endpoint);
    assert_eq!(
        result_event.and_then(|event| event.local_endpoint.as_deref()),
        Some(process_plan.local_endpoint.as_str())
    );
    assert_eq!(
        result_event.and_then(|event| event.pid),
        Some(outcome.process.pid)
    );
    assert!(!stopped);
}

#[tokio::test(start_paused = true)]
async fn i023_drop_with_active_monitor_cancels_tasks_without_stopping_process() {
    let processes = Arc::new(ScriptedProcessManager::new());
    let readiness = Arc::new(ScriptedReadinessProbe::default());
    let health = Arc::new(ScriptedHealthProbe::default());
    let (service, mut errors) = new_service(
        Arc::clone(&processes),
        readiness,
        health,
        Arc::new(RecordingPublisher::default()),
    );
    deploy(&service, "model-i023", Arc::new(FixedAdapter::new(plan())))
        .await
        .unwrap();
    assert_eq!(Arc::strong_count(&processes), 2);

    drop(service);
    assert_eq!(
        timeout(Duration::from_secs(1), errors.recv())
            .await
            .unwrap(),
        None
    );
    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    assert_eq!(processes.stop_count().await, 0);
    assert_eq!(processes.liveness_calls.load(Ordering::SeqCst), 0);
    assert_eq!(Arc::strong_count(&processes), 1);
}

#[tokio::test]
async fn d001_real_process_first_health_deployment_returns_ready() {
    let (processes, service, publisher) = live_service();
    let (process_plan, log_path) = python_plan(&[200]);
    let adapter = Arc::new(FixedAdapter::new(process_plan.clone()));
    let outcome = deploy(&service, "model-d001", adapter).await.unwrap();
    let alive = processes.is_running(outcome.process).await.unwrap();
    let events = publisher.received.lock().await.clone();
    let result = events
        .iter()
        .find(|event| event.kind == DeploymentEventKind::InitialResult(NodeState::Ready));
    processes.stop(outcome.process).await.unwrap();
    let stopped = processes.is_running(outcome.process).await.unwrap();
    remove_log(&log_path);

    assert_eq!(outcome.state, NodeState::Ready);
    assert_ne!(outcome.process.pid, 0);
    assert_eq!(outcome.plan, process_plan);
    assert!(alive);
    assert_eq!(
        result.and_then(|event| event.pid),
        Some(outcome.process.pid)
    );
    assert_eq!(
        result.and_then(|event| event.local_endpoint.as_deref()),
        Some(process_plan.local_endpoint.as_str())
    );
    assert!(!stopped);
}

#[tokio::test]
async fn d002_first_health_non_2xx_keeps_the_real_process_alive() {
    let (processes, service, publisher) = live_service();
    let (process_plan, log_path) = python_plan(&[200, 503]);
    let outcome = deploy(
        &service,
        "model-d002",
        Arc::new(FixedAdapter::new(process_plan.clone())),
    )
    .await
    .unwrap();
    let alive = processes.is_running(outcome.process).await.unwrap();
    let events = publisher.received.lock().await.clone();
    let result = events
        .iter()
        .find(|event| event.kind == DeploymentEventKind::InitialResult(NodeState::Unhealthy));
    processes.stop(outcome.process).await.unwrap();
    let stopped = processes.is_running(outcome.process).await.unwrap();
    remove_log(&log_path);

    assert_eq!(outcome.state, NodeState::Unhealthy);
    assert!(alive);
    assert_eq!(
        result.and_then(|event| event.local_endpoint.as_deref()),
        Some(process_plan.local_endpoint.as_str())
    );
    assert!(!stopped);
}

#[tokio::test]
async fn d003_readiness_retries_after_non_2xx_then_recovers() {
    let (processes, service, publisher) = live_service();
    let (process_plan, log_path) = python_plan(&[503, 200, 200]);
    let outcome = deploy(
        &service,
        "model-d003",
        Arc::new(FixedAdapter::new(process_plan.clone())),
    )
    .await
    .unwrap();
    let alive = processes.is_running(outcome.process).await.unwrap();
    let requested_paths = std::fs::read_to_string(&log_path).unwrap();
    let events = publisher.received.lock().await.clone();
    let result = events
        .iter()
        .find(|event| event.kind == DeploymentEventKind::InitialResult(NodeState::Ready));
    processes.stop(outcome.process).await.unwrap();
    let stopped = processes.is_running(outcome.process).await.unwrap();
    remove_log(&log_path);

    assert_eq!(outcome.state, NodeState::Ready);
    assert!(alive);
    assert!(requested_paths.lines().count() >= 3);
    assert_eq!(
        result.and_then(|event| event.local_endpoint.as_deref()),
        Some(process_plan.local_endpoint.as_str())
    );
    assert!(!stopped);
}

#[tokio::test]
async fn d004_replacement_stops_old_process_before_starting_new_plan() {
    let (processes, service, publisher) = live_service();
    let (old_plan, old_log) = python_plan(&[200]);
    let (new_plan, new_log) = python_plan(&[200]);
    let old = deploy(
        &service,
        "model-d004",
        Arc::new(FixedAdapter::new(old_plan)),
    )
    .await
    .unwrap();
    let replacement = deploy(
        &service,
        "model-d004",
        Arc::new(FixedAdapter::new(new_plan.clone())),
    )
    .await
    .unwrap();
    let old_alive = processes.is_running(old.process).await.unwrap();
    let new_alive = processes.is_running(replacement.process).await.unwrap();
    let events = publisher.received.lock().await.clone();
    let stopping = events
        .iter()
        .position(|event| {
            event.kind == DeploymentEventKind::Progress(DeploymentStage::StoppingOldRuntime)
        })
        .unwrap();
    let planning = events
        .iter()
        .enumerate()
        .skip(stopping + 1)
        .find(|(_, event)| event.kind == DeploymentEventKind::Progress(DeploymentStage::Planning))
        .map(|(index, _)| index)
        .unwrap();
    let starting = events
        .iter()
        .enumerate()
        .skip(planning + 1)
        .find(|(_, event)| event.kind == DeploymentEventKind::Progress(DeploymentStage::Starting))
        .map(|(index, _)| index)
        .unwrap();
    processes.stop(replacement.process).await.unwrap();
    let new_stopped = processes.is_running(replacement.process).await.unwrap();
    remove_log(&old_log);
    remove_log(&new_log);

    assert_eq!(replacement.state, NodeState::Ready);
    assert!(!old_alive);
    assert!(new_alive);
    assert_eq!(replacement.plan, new_plan);
    assert!(stopping < planning && planning < starting);
    assert!(!new_stopped);
}

#[tokio::test]
async fn d005_explicit_stop_ends_the_deployed_process() {
    let (processes, service, _publisher) = live_service();
    let (process_plan, log_path) = python_plan(&[200]);
    let outcome = deploy(
        &service,
        "model-d005",
        Arc::new(FixedAdapter::new(process_plan)),
    )
    .await
    .unwrap();
    processes.stop(outcome.process).await.unwrap();
    let stopped = processes.is_running(outcome.process).await.unwrap();
    processes.stop(outcome.process).await.unwrap();
    remove_log(&log_path);

    assert!(!stopped);
}
