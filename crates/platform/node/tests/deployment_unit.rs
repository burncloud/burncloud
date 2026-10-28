#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "测试断言失败时需要立即暴露场景错误。"
)]

#[path = "support/deployment.rs"]
mod support;

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use burncloud_node_runtime::{
    deployment::{
        DeploymentErrorKind, DeploymentEvent, DeploymentEventKind, DeploymentMonitorError,
        DeploymentStage, HttpProbe, LocalProcessManager,
    },
    FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe, FakeProcessManager,
    FakeReadinessProbe, FakeRuntimePreparer, HealthError, HealthProbe, InvalidNodeTransition,
    NodeComposition, NodeState, NodeStateMachine, ProcessError, ProcessHandle, ProcessManager,
    ProcessSpec, ReadinessError, ReadinessProbe, ReadinessTarget, ReconcileAction,
};
use support::{
    artifact, deploy, new_service, plan, FixedAdapter, Gate, RecordingPublisher,
    ScriptedHealthProbe, ScriptedProcessManager, ScriptedReadinessProbe,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::mpsc,
    task::JoinHandle,
    time::timeout,
};

async fn http_server(statuses: Vec<u16>) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        for status in statuses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 1024];
            let _ = stream.read(&mut request).await.unwrap();
            let reason = if status < 400 { "OK" } else { "Unavailable" };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        }
    });
    (format!("http://{address}/health"), task)
}

fn closed_monitor_errors() -> mpsc::UnboundedReceiver<DeploymentMonitorError> {
    let (_, receiver) = mpsc::unbounded_channel();
    receiver
}

#[tokio::test]
async fn u001_public_types_and_state_error_display_match_the_contract() {
    let stages = [
        DeploymentStage::Queued,
        DeploymentStage::StoppingOldRuntime,
        DeploymentStage::Planning,
        DeploymentStage::Starting,
        DeploymentStage::WaitingReady,
        DeploymentStage::HealthChecking,
    ];
    assert_eq!(stages[0], DeploymentStage::Queued);
    let event = DeploymentEvent {
        model_id: "model-u001".into(),
        kind: DeploymentEventKind::Progress(DeploymentStage::Queued),
        state: None,
        error_kind: None,
        local_endpoint: None,
        pid: None,
        error: None,
        cleanup_error: None,
    };
    assert_eq!(event.model_id, "model-u001");
    let invalid = InvalidNodeTransition {
        from: NodeState::Absent,
        to: NodeState::Ready,
    };
    assert!(invalid.to_string().contains("Absent"));
    assert!(invalid.to_string().contains("Ready"));
    let error = burncloud_node_runtime::deployment::DeploymentError {
        kind: DeploymentErrorKind::PlanFailed,
        message: "no plan".into(),
        cleanup_error: None,
        notification_error: None,
        preserved_state: None,
    };
    assert!(error.to_string().contains("PlanFailed"));
}

#[tokio::test]
async fn u002_service_drop_closes_monitor_error_channel_without_stopping_processes() {
    let processes = Arc::new(ScriptedProcessManager::new());
    let (service, mut errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        Arc::new(ScriptedHealthProbe::default()),
        Arc::new(RecordingPublisher::default()),
    );
    drop(service);
    assert_eq!(
        timeout(Duration::from_secs(1), errors.recv())
            .await
            .unwrap(),
        None
    );
    assert_eq!(processes.stop_count().await, 0);
}

#[tokio::test]
async fn u003_composition_starts_context_and_exposes_injected_capabilities() {
    let mut node = NodeComposition::start(
        FakeHardwareProbe::default(),
        FakeArtifactPreparer,
        FakeRuntimePreparer,
        Arc::new(FakeProcessManager::new(3103)),
        Arc::new(FakeReadinessProbe),
        Arc::new(FakeHealthProbe),
        closed_monitor_errors(),
    );
    assert!(node.context().started());
    assert_eq!(node.reconciler().state(), NodeState::Absent);
    assert_eq!(
        node.reconciler_mut().next_action().unwrap(),
        ReconcileAction::Resolve
    );
    assert_eq!(
        node.processes()
            .start(ProcessSpec {
                program: "ignored".into(),
                args: vec![],
            })
            .await
            .unwrap()
            .pid,
        3103
    );
    assert!(node
        .health()
        .is_healthy(ReadinessTarget {
            endpoint: "http://unused".into(),
        })
        .await
        .unwrap());
}

#[tokio::test]
async fn u004_composition_forwards_monitor_error_and_channel_closure() {
    let (sender, receiver) = mpsc::unbounded_channel();
    let mut node = NodeComposition::start(
        FakeHardwareProbe::default(),
        FakeArtifactPreparer,
        FakeRuntimePreparer,
        Arc::new(FakeProcessManager::new(3104)),
        Arc::new(FakeReadinessProbe),
        Arc::new(FakeHealthProbe),
        receiver,
    );
    let error = DeploymentMonitorError {
        model_id: "model-u004".into(),
        pid: 3104,
        kind: DeploymentErrorKind::ProcessExited,
        message: "exited".into(),
        cleanup_error: None,
    };
    sender.send(error.clone()).unwrap();
    assert_eq!(node.next_deployment_monitor_error().await, Some(error));
    drop(sender);
    assert_eq!(node.next_deployment_monitor_error().await, None);
}

#[tokio::test]
async fn u005_default_process_manager_starts_and_stops_a_real_child() {
    let manager = LocalProcessManager::default();
    let handle = manager
        .start(ProcessSpec {
            program: "python3".into(),
            args: vec!["-c".into(), "import time; time.sleep(30)".into()],
        })
        .await
        .unwrap();
    assert_ne!(handle.pid, 0);
    assert!(manager.is_running(handle).await.unwrap());
    manager.stop(handle).await.unwrap();
    assert!(!manager.is_running(handle).await.unwrap());
}

#[tokio::test]
async fn u006_default_http_probe_reports_a_real_loopback_2xx_as_healthy() {
    let (endpoint, server) = http_server(vec![200]).await;
    let probe = HttpProbe::default();
    assert!(probe
        .is_healthy(ReadinessTarget { endpoint })
        .await
        .unwrap());
    server.await.unwrap();
}

#[tokio::test]
async fn u007_deploy_returns_ready_with_the_adapter_plan_and_process_handle() {
    let processes = Arc::new(ScriptedProcessManager::new());
    let (service, _errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        Arc::new(ScriptedHealthProbe::default()),
        Arc::new(RecordingPublisher::default()),
    );
    let adapter = Arc::new(FixedAdapter::new(plan()));
    let outcome = deploy(&service, "model-u007", adapter).await.unwrap();
    assert_eq!(outcome.state, NodeState::Ready);
    assert_eq!(outcome.plan, plan());
    assert_eq!(outcome.process.pid, 41);
}

#[tokio::test]
async fn u008_runtime_adapter_inputs_and_plan_are_forwarded_unchanged() {
    let processes = Arc::new(ScriptedProcessManager::new());
    let (service, _errors) = new_service(
        processes,
        Arc::new(ScriptedReadinessProbe::default()),
        Arc::new(ScriptedHealthProbe::default()),
        Arc::new(RecordingPublisher::default()),
    );
    let adapter = Arc::new(FixedAdapter::new(plan()));
    let outcome = deploy(&service, "model-u008", adapter.clone())
        .await
        .unwrap();
    let inputs = adapter.inputs.lock().await;
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].0.executable, "runtime-prepared");
    assert_eq!(inputs[0].1, artifact());
    assert_eq!(outcome.plan, plan());
}

#[tokio::test]
async fn u009_process_start_uses_the_supplied_program_and_arguments() {
    let manager = LocalProcessManager::default();
    let handle = manager
        .start(ProcessSpec {
            program: "python3".into(),
            args: vec![
                "-c".into(),
                "import sys,time; assert sys.argv[1] == 'argument-check'; time.sleep(30)".into(),
                "argument-check".into(),
            ],
        })
        .await
        .unwrap();
    assert_ne!(handle.pid, 0);
    assert!(manager.is_running(handle).await.unwrap());
    manager.stop(handle).await.unwrap();
}

#[tokio::test]
async fn u010_process_stop_is_idempotent_and_rejects_zero_pid() {
    let manager = LocalProcessManager::default();
    let handle = manager
        .start(ProcessSpec {
            program: "python3".into(),
            args: vec!["-c".into(), "import time; time.sleep(30)".into()],
        })
        .await
        .unwrap();
    manager.stop(handle).await.unwrap();
    manager.stop(handle).await.unwrap();
    assert!(matches!(
        manager.stop(ProcessHandle { pid: 0 }).await,
        Err(ProcessError::StopFailed(_))
    ));
}

struct DefaultLivenessManager;

#[async_trait]
impl ProcessManager for DefaultLivenessManager {
    async fn start(&self, _spec: ProcessSpec) -> Result<ProcessHandle, ProcessError> {
        Ok(ProcessHandle { pid: 81 })
    }

    async fn stop(&self, _handle: ProcessHandle) -> Result<(), ProcessError> {
        Ok(())
    }
}

#[tokio::test]
async fn u011_liveness_checks_registered_child_and_trait_default_error() {
    let manager = LocalProcessManager::default();
    let handle = manager
        .start(ProcessSpec {
            program: "python3".into(),
            args: vec!["-c".into(), "import time; time.sleep(30)".into()],
        })
        .await
        .unwrap();
    assert!(manager.is_running(handle).await.unwrap());
    manager.stop(handle).await.unwrap();
    assert!(!manager.is_running(handle).await.unwrap());
    assert!(matches!(
        manager.is_running(ProcessHandle { pid: 0 }).await,
        Err(ProcessError::IsRunningFailed(_))
    ));
    assert!(matches!(
        DefaultLivenessManager
            .is_running(ProcessHandle { pid: 81 })
            .await,
        Err(ProcessError::IsRunningFailed(_))
    ));
}

#[tokio::test]
async fn u012_readiness_retries_non_2xx_and_rejects_an_invalid_url() {
    let (endpoint, server) = http_server(vec![503, 200]).await;
    let probe = HttpProbe::default();
    probe
        .wait_ready(ReadinessTarget { endpoint })
        .await
        .unwrap();
    server.await.unwrap();
    assert!(matches!(
        probe
            .wait_ready(ReadinessTarget {
                endpoint: "http://[invalid".into()
            })
            .await,
        Err(ReadinessError::CheckFailed(_))
    ));
}

#[tokio::test]
async fn u013_health_probe_distinguishes_2xx_non_2xx_and_invalid_urls() {
    let (healthy_url, healthy_server) = http_server(vec![200]).await;
    let (unhealthy_url, unhealthy_server) = http_server(vec![503]).await;
    let probe = HttpProbe::default();
    assert!(probe
        .is_healthy(ReadinessTarget {
            endpoint: healthy_url
        })
        .await
        .unwrap());
    assert!(!probe
        .is_healthy(ReadinessTarget {
            endpoint: unhealthy_url
        })
        .await
        .unwrap());
    healthy_server.await.unwrap();
    unhealthy_server.await.unwrap();
    let result = probe
        .is_healthy(ReadinessTarget {
            endpoint: "http://[invalid".into(),
        })
        .await;
    assert!(
        matches!(result, Err(HealthError::CheckFailed(_))),
        "非法 URL 的探测结果不符合预期：{result:?}"
    );
}

#[tokio::test]
async fn u014_event_publisher_receives_public_deployment_events() {
    let publisher = Arc::new(RecordingPublisher::default());
    let (service, _errors) = new_service(
        Arc::new(ScriptedProcessManager::new()),
        Arc::new(ScriptedReadinessProbe::default()),
        Arc::new(ScriptedHealthProbe::default()),
        Arc::clone(&publisher),
    );
    deploy(&service, "model-u014", Arc::new(FixedAdapter::new(plan())))
        .await
        .unwrap();
    let events = publisher.received.lock().await;
    assert!(events.iter().any(|event| {
        event.model_id == "model-u014"
            && event.kind == DeploymentEventKind::Progress(DeploymentStage::Queued)
    }));
    assert!(events.iter().any(|event| {
        event.kind == DeploymentEventKind::InitialResult(NodeState::Ready)
            && event.local_endpoint.as_deref() == Some("http://127.0.0.1:39122")
            && event.pid == Some(41)
    }));
}

#[tokio::test]
async fn u015_plan_and_start_errors_keep_their_classification() {
    let processes = Arc::new(ScriptedProcessManager::new());
    let publisher = Arc::new(RecordingPublisher::default());
    let (service, _errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        Arc::new(ScriptedHealthProbe::default()),
        Arc::clone(&publisher),
    );
    let plan_error = deploy(
        &service,
        "model-u015-plan",
        Arc::new(FixedAdapter::failing("no plan")),
    )
    .await
    .unwrap_err();
    assert_eq!(plan_error.kind, DeploymentErrorKind::PlanFailed);
    assert_eq!(processes.start_count().await, 0);

    *processes.start_error.lock().await = Some("cannot start".into());
    let start_error = deploy(
        &service,
        "model-u015-start",
        Arc::new(FixedAdapter::new(plan())),
    )
    .await
    .unwrap_err();
    assert_eq!(start_error.kind, DeploymentErrorKind::ProcessStartFailed);
    assert_eq!(processes.stop_count().await, 0);
    let events = publisher.received.lock().await;
    assert!(events.iter().any(|event| {
        event.kind == DeploymentEventKind::DeploymentFailed
            && event.error_kind == Some(DeploymentErrorKind::PlanFailed)
    }));
}

#[tokio::test]
async fn u016_probe_failures_clean_up_and_preserve_the_primary_error() {
    let processes = Arc::new(ScriptedProcessManager::new());
    *processes.stop_error.lock().await = Some("cleanup denied".into());
    let readiness = Arc::new(ScriptedReadinessProbe::default());
    readiness
        .results
        .lock()
        .await
        .push_back(Err("not ready".into()));
    let (service, _errors) = new_service(
        Arc::clone(&processes),
        readiness,
        Arc::new(ScriptedHealthProbe::default()),
        Arc::new(RecordingPublisher::default()),
    );
    let error = deploy(
        &service,
        "model-u016-readiness",
        Arc::new(FixedAdapter::new(plan())),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind, DeploymentErrorKind::ReadinessFailed);
    assert_eq!(
        error.cleanup_error.as_deref(),
        Some("process stop failed: cleanup denied")
    );
    assert_eq!(processes.stop_count().await, 1);

    let processes = Arc::new(ScriptedProcessManager::new());
    *processes.stop_error.lock().await = Some("cleanup denied".into());
    let health = Arc::new(ScriptedHealthProbe::default());
    health
        .results
        .lock()
        .await
        .push_back(Err("probe failed".into()));
    let (service, _errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        health,
        Arc::new(RecordingPublisher::default()),
    );
    let error = deploy(
        &service,
        "model-u016-health",
        Arc::new(FixedAdapter::new(plan())),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind, DeploymentErrorKind::HealthProbeFailed);
    assert_eq!(
        error.cleanup_error.as_deref(),
        Some("process stop failed: cleanup denied")
    );
}

#[tokio::test]
async fn u017_publisher_retries_the_same_event_once() {
    let publisher = Arc::new(RecordingPublisher::default());
    publisher
        .fail_kind(DeploymentEventKind::Progress(DeploymentStage::Queued), 1)
        .await;
    let (service, _errors) = new_service(
        Arc::new(ScriptedProcessManager::new()),
        Arc::new(ScriptedReadinessProbe::default()),
        Arc::new(ScriptedHealthProbe::default()),
        Arc::clone(&publisher),
    );
    deploy(&service, "model-u017", Arc::new(FixedAdapter::new(plan())))
        .await
        .unwrap();
    let attempts = publisher.attempts.lock().await;
    let queued: Vec<_> = attempts
        .iter()
        .filter(|event| event.kind == DeploymentEventKind::Progress(DeploymentStage::Queued))
        .collect();
    assert_eq!(queued.len(), 2);
    assert_eq!(queued[0], queued[1]);
}

#[tokio::test]
async fn u018_publish_exhaustion_clears_in_flight_and_cleans_started_process() {
    let publisher = Arc::new(RecordingPublisher::default());
    publisher
        .fail_kind(DeploymentEventKind::Progress(DeploymentStage::Queued), 2)
        .await;
    let processes = Arc::new(ScriptedProcessManager::new());
    let (service, _errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        Arc::new(ScriptedHealthProbe::default()),
        Arc::clone(&publisher),
    );
    let adapter = Arc::new(FixedAdapter::new(plan()));
    assert_eq!(
        deploy(&service, "model-u018-queue", adapter.clone())
            .await
            .unwrap_err()
            .kind,
        DeploymentErrorKind::EventPublishFailed
    );
    assert_eq!(adapter.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(processes.start_count().await, 0);
    deploy(&service, "model-u018-queue", adapter.clone())
        .await
        .unwrap();
    assert_eq!(adapter.calls.load(std::sync::atomic::Ordering::SeqCst), 1);

    publisher
        .fail_kind(
            DeploymentEventKind::Progress(DeploymentStage::WaitingReady),
            2,
        )
        .await;
    let error = deploy(
        &service,
        "model-u018-progress",
        Arc::new(FixedAdapter::new(plan())),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind, DeploymentErrorKind::EventPublishFailed);
    assert_eq!(processes.stop_count().await, 1);
}

#[tokio::test]
async fn u019_same_model_in_flight_calls_share_the_canonical_result() {
    let gate = Arc::new(Gate::default());
    let readiness = Arc::new(ScriptedReadinessProbe::default());
    *readiness.gate.lock().await = Some(Arc::clone(&gate));
    let processes = Arc::new(ScriptedProcessManager::new());
    let (service, _errors) = new_service(
        Arc::clone(&processes),
        readiness,
        Arc::new(ScriptedHealthProbe::default()),
        Arc::new(RecordingPublisher::default()),
    );
    let service = Arc::new(service);
    let first_adapter = Arc::new(FixedAdapter::new(plan()));
    let first_service = Arc::clone(&service);
    let first =
        tokio::spawn(async move { deploy(&first_service, "model-u019", first_adapter).await });
    gate.wait_until_entered().await;
    let second_service = Arc::clone(&service);
    let second_adapter = Arc::new(FixedAdapter::new(plan()));
    let second =
        tokio::spawn(async move { deploy(&second_service, "model-u019", second_adapter).await });
    assert_eq!(processes.start_count().await, 1);
    gate.open();
    let first = first.await.unwrap().unwrap();
    let second = second.await.unwrap().unwrap();
    assert_eq!(first, second);
    assert_eq!(processes.start_count().await, 1);
}

#[tokio::test]
async fn u020_different_models_execute_in_fifo_order() {
    let gate = Arc::new(Gate::default());
    let processes = Arc::new(ScriptedProcessManager::new());
    let (service, _errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        Arc::new(ScriptedHealthProbe::default()),
        Arc::new(RecordingPublisher::default()),
    );
    let service = Arc::new(service);
    let mut plan_a = plan();
    plan_a.process.program = "runtime-A".into();
    let adapter_a = Arc::new(FixedAdapter::gated(plan_a, Arc::clone(&gate)));
    let first_service = Arc::clone(&service);
    let first_adapter = Arc::clone(&adapter_a);
    let first = tokio::spawn(async move { deploy(&first_service, "model-A", first_adapter).await });
    gate.wait_until_entered().await;
    let mut plan_b = plan();
    plan_b.process.program = "runtime-B".into();
    let adapter_b = Arc::new(FixedAdapter::new(plan_b));
    let second_service = Arc::clone(&service);
    let second_adapter = Arc::clone(&adapter_b);
    let second =
        tokio::spawn(async move { deploy(&second_service, "model-B", second_adapter).await });
    assert_eq!(adapter_b.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    gate.open();
    first.await.unwrap().unwrap();
    second.await.unwrap().unwrap();
    let starts = processes.starts.lock().await;
    assert_eq!(starts[0].program, "runtime-A");
    assert_eq!(starts[1].program, "runtime-B");
}

#[tokio::test]
async fn u021_completed_model_requests_create_a_new_deployment() {
    let processes = Arc::new(ScriptedProcessManager::new());
    let (service, _errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        Arc::new(ScriptedHealthProbe::default()),
        Arc::new(RecordingPublisher::default()),
    );
    let adapter = Arc::new(FixedAdapter::new(plan()));
    let first = deploy(&service, "model-u021", adapter.clone())
        .await
        .unwrap();
    let second = deploy(&service, "model-u021", adapter.clone())
        .await
        .unwrap();
    assert_ne!(first.process, second.process);
    assert_eq!(adapter.calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(processes.start_count().await, 2);
    assert_eq!(processes.stop_count().await, 1);
}

#[tokio::test]
async fn u022_replacement_stops_old_runtime_before_planning_new_runtime() {
    let publisher = Arc::new(RecordingPublisher::default());
    let processes = Arc::new(ScriptedProcessManager::new());
    let (service, _errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        Arc::new(ScriptedHealthProbe::default()),
        Arc::clone(&publisher),
    );
    deploy(&service, "model-u022", Arc::new(FixedAdapter::new(plan())))
        .await
        .unwrap();
    deploy(&service, "model-u022", Arc::new(FixedAdapter::new(plan())))
        .await
        .unwrap();
    assert_eq!(processes.stop_count().await, 1);
    let events = publisher.received.lock().await;
    let stopping = events
        .iter()
        .position(|event| {
            event.kind == DeploymentEventKind::Progress(DeploymentStage::StoppingOldRuntime)
        })
        .unwrap();
    let planning_after = events
        .iter()
        .enumerate()
        .skip(stopping + 1)
        .find(|(_, event)| event.kind == DeploymentEventKind::Progress(DeploymentStage::Planning))
        .map(|(index, _)| index)
        .unwrap();
    assert!(stopping < planning_after);
}

#[tokio::test(start_paused = true)]
async fn u023_old_stop_failure_preserves_state_and_restarts_its_monitor() {
    let processes = Arc::new(ScriptedProcessManager::new());
    let health = Arc::new(ScriptedHealthProbe::default());
    let (service, _errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        Arc::clone(&health),
        Arc::new(RecordingPublisher::default()),
    );
    deploy(&service, "model-u023", Arc::new(FixedAdapter::new(plan())))
        .await
        .unwrap();
    *processes.stop_error.lock().await = Some("stop failed".into());
    let replacement = Arc::new(FixedAdapter::new(plan()));
    let error = deploy(&service, "model-u023", replacement.clone())
        .await
        .unwrap_err();
    assert_eq!(error.kind, DeploymentErrorKind::OldRuntimeStopFailed);
    assert_eq!(error.preserved_state, Some(NodeState::Ready));
    assert_eq!(
        replacement.calls.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;
    assert_eq!(health.targets.lock().await.len(), 2);
}

#[tokio::test(start_paused = true)]
async fn u024_monitor_publishes_only_state_changes_and_keeps_unhealthy_process() {
    let processes = Arc::new(ScriptedProcessManager::new());
    let health = Arc::new(ScriptedHealthProbe::default());
    health
        .results
        .lock()
        .await
        .extend([Ok(true), Ok(false), Ok(false), Ok(true)]);
    let publisher = Arc::new(RecordingPublisher::default());
    let (service, _errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        health,
        Arc::clone(&publisher),
    );
    let outcome = deploy(&service, "model-u024", Arc::new(FixedAdapter::new(plan())))
        .await
        .unwrap();
    for _ in 0..3 {
        tokio::time::advance(Duration::from_secs(10)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;
    }
    let events = publisher.received.lock().await;
    let changes: Vec<_> = events
        .iter()
        .filter_map(|event| match event.kind {
            DeploymentEventKind::StateChanged(state) => Some(state),
            _ => None,
        })
        .collect();
    assert_eq!(changes, vec![NodeState::Unhealthy, NodeState::Ready]);
    assert_eq!(processes.stop_count().await, 0);
    assert_eq!(outcome.state, NodeState::Ready);
}

#[tokio::test(start_paused = true)]
async fn u025_monitor_reports_exit_liveness_and_health_failures() {
    let processes = Arc::new(ScriptedProcessManager::new());
    processes.liveness.lock().await.push_back(Ok(false));
    let publisher = Arc::new(RecordingPublisher::default());
    let (service, mut monitor_errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        Arc::new(ScriptedHealthProbe::default()),
        Arc::clone(&publisher),
    );
    deploy(
        &service,
        "model-u025-exit",
        Arc::new(FixedAdapter::new(plan())),
    )
    .await
    .unwrap();
    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    let events = publisher.received.lock().await;
    assert!(events.iter().any(|event| {
        event.kind == DeploymentEventKind::StateChanged(NodeState::Failed)
            && event.error_kind == Some(DeploymentErrorKind::ProcessExited)
    }));
    assert_eq!(processes.stop_count().await, 0);
    assert!(monitor_errors.try_recv().is_err());

    let processes = Arc::new(ScriptedProcessManager::new());
    processes
        .liveness
        .lock()
        .await
        .push_back(Err("query failed".into()));
    let (service, _errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        Arc::new(ScriptedHealthProbe::default()),
        Arc::new(RecordingPublisher::default()),
    );
    deploy(
        &service,
        "model-u025-liveness",
        Arc::new(FixedAdapter::new(plan())),
    )
    .await
    .unwrap();
    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    assert_eq!(processes.stop_count().await, 1);

    let processes = Arc::new(ScriptedProcessManager::new());
    let health = Arc::new(ScriptedHealthProbe::default());
    health
        .results
        .lock()
        .await
        .extend([Ok(true), Err("health failed".into())]);
    let (service, _errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        health,
        Arc::new(RecordingPublisher::default()),
    );
    deploy(
        &service,
        "model-u025-health",
        Arc::new(FixedAdapter::new(plan())),
    )
    .await
    .unwrap();
    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    assert_eq!(processes.stop_count().await, 1);
}

#[tokio::test(start_paused = true)]
async fn u026_monitor_error_channel_receives_only_failed_notification_exhaustion() {
    let processes = Arc::new(ScriptedProcessManager::new());
    processes.liveness.lock().await.push_back(Ok(false));
    let publisher = Arc::new(RecordingPublisher::default());
    let (service, mut errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        Arc::new(ScriptedHealthProbe::default()),
        Arc::clone(&publisher),
    );
    deploy(
        &service,
        "model-u026-ok",
        Arc::new(FixedAdapter::new(plan())),
    )
    .await
    .unwrap();
    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    assert!(errors.try_recv().is_err());

    let processes = Arc::new(ScriptedProcessManager::new());
    processes.liveness.lock().await.push_back(Ok(false));
    let publisher = Arc::new(RecordingPublisher::default());
    publisher
        .fail_kind(DeploymentEventKind::StateChanged(NodeState::Failed), 2)
        .await;
    let (service, mut errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        Arc::new(ScriptedHealthProbe::default()),
        publisher,
    );
    deploy(
        &service,
        "model-u026-fail",
        Arc::new(FixedAdapter::new(plan())),
    )
    .await
    .unwrap();
    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    let error = errors.recv().await.unwrap();
    assert_eq!(error.model_id, "model-u026-fail");
    assert_eq!(error.pid, 41);
    assert_eq!(error.kind, DeploymentErrorKind::ProcessExited);
    assert!(error.message.contains("failed to publish Failed state"));
}

#[tokio::test(start_paused = true)]
async fn u027_monitor_state_event_exhaustion_stops_process_and_reports_error() {
    let processes = Arc::new(ScriptedProcessManager::new());
    let health = Arc::new(ScriptedHealthProbe::default());
    health.results.lock().await.extend([Ok(true), Ok(false)]);
    let publisher = Arc::new(RecordingPublisher::default());
    publisher
        .fail_kind(DeploymentEventKind::StateChanged(NodeState::Unhealthy), 2)
        .await;
    let (service, mut errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        health,
        publisher,
    );
    deploy(&service, "model-u027", Arc::new(FixedAdapter::new(plan())))
        .await
        .unwrap();
    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;
    assert_eq!(processes.stop_count().await, 1);
    assert_eq!(
        errors.recv().await.unwrap().kind,
        DeploymentErrorKind::EventPublishFailed
    );
}

#[tokio::test]
async fn u028_cancelled_waiter_does_not_cancel_the_canonical_deployment() {
    let gate = Arc::new(Gate::default());
    let readiness = Arc::new(ScriptedReadinessProbe::default());
    *readiness.gate.lock().await = Some(Arc::clone(&gate));
    let processes = Arc::new(ScriptedProcessManager::new());
    let (service, _errors) = new_service(
        Arc::clone(&processes),
        readiness,
        Arc::new(ScriptedHealthProbe::default()),
        Arc::new(RecordingPublisher::default()),
    );
    let service = Arc::new(service);
    let first_service = Arc::clone(&service);
    let cancelled = tokio::spawn(async move {
        deploy(
            &first_service,
            "model-u028",
            Arc::new(FixedAdapter::new(plan())),
        )
        .await
    });
    gate.wait_until_entered().await;
    let second_service = Arc::clone(&service);
    let waiting = tokio::spawn(async move {
        deploy(
            &second_service,
            "model-u028",
            Arc::new(FixedAdapter::new(plan())),
        )
        .await
    });
    cancelled.abort();
    gate.open();
    assert!(waiting.await.unwrap().is_ok());
    assert_eq!(processes.start_count().await, 1);
}

#[tokio::test]
async fn u029_queue_worker_waits_for_successful_queued_event() {
    let gate = Arc::new(Gate::default());
    let publisher = Arc::new(RecordingPublisher::default());
    publisher
        .block_kind(
            DeploymentEventKind::Progress(DeploymentStage::Queued),
            Arc::clone(&gate),
        )
        .await;
    let processes = Arc::new(ScriptedProcessManager::new());
    let (service, _errors) = new_service(
        Arc::clone(&processes),
        Arc::new(ScriptedReadinessProbe::default()),
        Arc::new(ScriptedHealthProbe::default()),
        publisher,
    );
    let service = Arc::new(service);
    let adapter = Arc::new(FixedAdapter::new(plan()));
    let task_service = Arc::clone(&service);
    let task_adapter = Arc::clone(&adapter);
    let task = tokio::spawn(async move { deploy(&task_service, "model-u029", task_adapter).await });
    gate.wait_until_entered().await;
    tokio::task::yield_now().await;
    assert_eq!(adapter.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(processes.start_count().await, 0);
    gate.open();
    task.await.unwrap().unwrap();
    assert_eq!(adapter.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn u030_state_machine_accepts_deployment_edges_and_refuses_invalid_reverse_edges() {
    assert!(NodeState::WaitingReady.can_transition_to(NodeState::Unhealthy));
    assert!(NodeState::Unhealthy.can_transition_to(NodeState::Ready));
    assert!(!NodeState::Ready.can_transition_to(NodeState::WaitingReady));
    assert!(NodeState::Routable.is_serving());
    assert!(NodeState::Unhealthy.is_failure());
    let mut machine = NodeStateMachine::new();
    let error = machine.transition(NodeState::Ready).unwrap_err();
    assert_eq!(
        error,
        InvalidNodeTransition {
            from: NodeState::Absent,
            to: NodeState::Ready,
        }
    );
    assert_eq!(machine.state(), NodeState::Absent);
}
