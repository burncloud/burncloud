use burncloud_node_runtime::{
    FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe, FakeProcessManager,
    FakeReadinessProbe, FakeRuntimeAdapter, FakeRuntimePreparer, HealthError, HealthProbe,
    NodeComposition, NodeState, PreparedArtifact, PreparedRuntime, ProcessError, ProcessHandle,
    ProcessManager, ProcessPlan, ProcessSpec, ReadinessError, ReadinessProbe, ReadinessTarget,
    RuntimeAdapter, RuntimeAdapterError,
};
use burncloud_router::local_attachment::LocalRouteAttachmentId;
use burncloud_server::node_attachment::{
    attach_ready_node_route, begin_detached_route_recovery, detach_unhealthy_node_route,
    prepare_and_attach_node_route, LocalRouteOutcome,
};
use burncloud_server::node_orchestrator::{ModelDemand, NodeOrchestrator};
use burncloud_server::node_request::NodeRequestState;
use burncloud_service_models::FakeModelResolver;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};

#[derive(Debug, Clone)]
struct RecordingRuntimeAdapter {
    inputs: Arc<Mutex<Option<(PreparedRuntime, PreparedArtifact)>>>,
    plan: ProcessPlan,
}

#[async_trait::async_trait]
impl RuntimeAdapter for RecordingRuntimeAdapter {
    async fn plan(
        &self,
        runtime: &PreparedRuntime,
        artifact: &PreparedArtifact,
    ) -> Result<ProcessPlan, RuntimeAdapterError> {
        *self.inputs.lock().unwrap() = Some((runtime.clone(), artifact.clone()));
        Ok(self.plan.clone())
    }
}

#[derive(Debug, Clone)]
struct FailingRuntimeAdapter {
    calls: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl RuntimeAdapter for FailingRuntimeAdapter {
    async fn plan(
        &self,
        _runtime: &PreparedRuntime,
        _artifact: &PreparedArtifact,
    ) -> Result<ProcessPlan, RuntimeAdapterError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(RuntimeAdapterError::PlanFailed(
            "intentional fake planning failure".into(),
        ))
    }
}

#[derive(Debug, Clone, Default)]
struct RecordingProcessManager {
    started: Arc<Mutex<Option<ProcessSpec>>>,
}

#[async_trait::async_trait]
impl ProcessManager for RecordingProcessManager {
    async fn start(&self, spec: ProcessSpec) -> Result<ProcessHandle, ProcessError> {
        *self.started.lock().unwrap() = Some(spec);
        Ok(ProcessHandle { pid: 574 })
    }

    async fn stop(&self, _handle: ProcessHandle) -> Result<(), ProcessError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
struct RecordingReadinessProbe {
    target: Arc<Mutex<Option<ReadinessTarget>>>,
}

#[async_trait::async_trait]
impl ReadinessProbe for RecordingReadinessProbe {
    async fn wait_ready(&self, target: ReadinessTarget) -> Result<(), ReadinessError> {
        *self.target.lock().unwrap() = Some(target);
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
struct RecordingHealthProbe {
    target: Arc<Mutex<Option<ReadinessTarget>>>,
}

#[async_trait::async_trait]
impl HealthProbe for RecordingHealthProbe {
    async fn is_healthy(&self, target: ReadinessTarget) -> Result<bool, HealthError> {
        *self.target.lock().unwrap() = Some(target);
        Ok(true)
    }
}

#[derive(Debug, Clone)]
struct PendingProcessManager {
    entered: Arc<AtomicBool>,
}

#[async_trait::async_trait]
impl ProcessManager for PendingProcessManager {
    async fn start(&self, _spec: ProcessSpec) -> Result<ProcessHandle, ProcessError> {
        self.entered.store(true, Ordering::SeqCst);
        std::future::pending().await
    }

    async fn stop(&self, _handle: ProcessHandle) -> Result<(), ProcessError> {
        Ok(())
    }
}

fn fake_orchestrator() -> NodeOrchestrator<
    FakeModelResolver,
    FakeRuntimeAdapter,
    FakeHardwareProbe,
    FakeArtifactPreparer,
    FakeRuntimePreparer,
    FakeProcessManager,
    FakeReadinessProbe,
    FakeHealthProbe,
> {
    NodeOrchestrator::new(
        FakeModelResolver,
        FakeRuntimeAdapter,
        NodeComposition::start(
            FakeHardwareProbe::default(),
            FakeArtifactPreparer,
            FakeRuntimePreparer,
            FakeProcessManager::default(),
            FakeReadinessProbe,
            FakeHealthProbe,
        ),
    )
}

/// #564 acceptance: desired demand + boundary side effects run the complete
/// lifecycle without tests manually injecting success evidence.
#[tokio::test]
async fn model_demand_runs_full_fake_route_failure_recovery_and_reattach_rail() {
    let request_state = NodeRequestState::default();
    let mut node = fake_orchestrator().with_request_state(request_state.clone());

    let first = prepare_and_attach_node_route(
        &mut node,
        ModelDemand::new("qwen-4b").unwrap(),
        |_, _| async { Ok(LocalRouteAttachmentId(101)) },
    )
    .await
    .unwrap();
    assert_eq!(
        first,
        LocalRouteOutcome::Routable(LocalRouteAttachmentId(101))
    );
    assert_eq!(node.workload_state("qwen-4b"), Some(NodeState::Routable));

    let quarantined = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let quarantine_flag = quarantined.clone();
    let detach_flag = quarantined.clone();
    let detached = detach_unhealthy_node_route(
        &mut node,
        "qwen-4b",
        LocalRouteAttachmentId(101),
        move |_| async move {
            quarantine_flag.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        },
        move |_| async move {
            assert!(detach_flag.load(std::sync::atomic::Ordering::SeqCst));
            Ok(())
        },
    )
    .await
    .unwrap();

    assert_eq!(node.workload_state("qwen-4b"), Some(NodeState::Unhealthy));
    assert_eq!(
        request_state.state_for("qwen-4b"),
        Some(NodeState::Unhealthy)
    );

    begin_detached_route_recovery(&mut node, "qwen-4b", detached).unwrap();
    assert_eq!(node.workload_state("qwen-4b"), Some(NodeState::Starting));

    node.recover_until_ready("qwen-4b").await.unwrap();
    assert_eq!(node.workload_state("qwen-4b"), Some(NodeState::Ready));

    let second_route = attach_ready_node_route(&mut node, "qwen-4b", || async {
        Ok(LocalRouteAttachmentId(202))
    })
    .await
    .unwrap();
    assert_eq!(second_route, LocalRouteAttachmentId(202));
    assert_eq!(node.workload_state("qwen-4b"), Some(NodeState::Routable));
    assert_eq!(
        request_state.state_for("qwen-4b"),
        Some(NodeState::Routable)
    );
}

#[tokio::test]
async fn two_models_keep_independent_state_plan_and_attachment() {
    let mut node = fake_orchestrator();

    prepare_and_attach_node_route(
        &mut node,
        ModelDemand::new("qwen-4b").unwrap(),
        |_, _| async { Ok(LocalRouteAttachmentId(301)) },
    )
    .await
    .unwrap();

    let qwen_plan = node.workload_plan("qwen-4b").cloned().unwrap();

    node.prepare_until_ready(ModelDemand::new("deepseek-8b").unwrap())
        .await
        .unwrap();

    assert_eq!(node.workload_count(), 2);
    assert_eq!(node.workload_state("qwen-4b"), Some(NodeState::Routable));
    assert_eq!(
        node.workload_attachment_id("qwen-4b"),
        Some(LocalRouteAttachmentId(301))
    );
    assert_eq!(node.workload_plan("qwen-4b"), Some(&qwen_plan));

    assert_eq!(node.workload_state("deepseek-8b"), Some(NodeState::Ready));
    assert_eq!(node.workload_attachment_id("deepseek-8b"), None);
}

#[tokio::test]
async fn repeated_routable_demand_does_not_attach_again() {
    let mut node = fake_orchestrator();
    let first = prepare_and_attach_node_route(
        &mut node,
        ModelDemand::new("qwen-4b").unwrap(),
        |_, _| async { Ok(LocalRouteAttachmentId(401)) },
    )
    .await
    .unwrap();
    assert_eq!(
        first,
        LocalRouteOutcome::Routable(LocalRouteAttachmentId(401))
    );

    let plan_before = node.workload_plan("qwen-4b").cloned().unwrap();
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let calls_for_attach = calls.clone();

    let repeated = prepare_and_attach_node_route(
        &mut node,
        ModelDemand::new("qwen-4b").unwrap(),
        move |_, _| async move {
            calls_for_attach.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(LocalRouteAttachmentId(999))
        },
    )
    .await
    .unwrap();

    assert_eq!(
        repeated,
        LocalRouteOutcome::Routable(LocalRouteAttachmentId(401))
    );
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(
        node.workload_attachment_id("qwen-4b"),
        Some(LocalRouteAttachmentId(401))
    );
    assert_eq!(node.workload_plan("qwen-4b"), Some(&plan_before));
    assert_eq!(node.workload_state("qwen-4b"), Some(NodeState::Routable));
}

#[tokio::test]
async fn non_ready_model_cannot_trigger_attach_side_effect() {
    let mut node = fake_orchestrator();
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let calls_for_attach = calls.clone();

    let result = attach_ready_node_route(&mut node, "qwen-4b", move || async move {
        calls_for_attach.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(LocalRouteAttachmentId(999))
    })
    .await;

    assert!(result.is_err());
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(node.workload_state("qwen-4b"), None);
    assert_eq!(node.workload_attachment_id("qwen-4b"), None);
}

#[tokio::test]
async fn attach_failure_stops_permanent_model_preparing() {
    let request_state = NodeRequestState::default();
    let mut node = fake_orchestrator().with_request_state(request_state.clone());

    node.prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
        .await
        .unwrap();
    assert!(request_state.route_miss_response("qwen-4b").is_some());

    let result = attach_ready_node_route(&mut node, "qwen-4b", || async {
        anyhow::bail!("traffic attach failed")
    })
    .await;

    assert!(result.is_err());
    assert_eq!(node.workload_state("qwen-4b"), Some(NodeState::Failed));
    assert!(request_state.route_miss_response("qwen-4b").is_none());
}

#[tokio::test]
async fn detached_route_receipt_cannot_recover_another_model() {
    let mut node = fake_orchestrator();

    let qwen_route = match prepare_and_attach_node_route(
        &mut node,
        ModelDemand::new("qwen-4b").unwrap(),
        |_, _| async { Ok(LocalRouteAttachmentId(501)) },
    )
    .await
    .unwrap()
    {
        LocalRouteOutcome::Routable(id) => id,
        LocalRouteOutcome::Unsupported(reason) => panic!("unexpected unsupported: {reason:?}"),
    };
    let qwen_receipt = detach_unhealthy_node_route(
        &mut node,
        "qwen-4b",
        qwen_route,
        |_| async { Ok(()) },
        |_| async { Ok(()) },
    )
    .await
    .unwrap();

    let deepseek_route = match prepare_and_attach_node_route(
        &mut node,
        ModelDemand::new("deepseek-8b").unwrap(),
        |_, _| async { Ok(LocalRouteAttachmentId(502)) },
    )
    .await
    .unwrap()
    {
        LocalRouteOutcome::Routable(id) => id,
        LocalRouteOutcome::Unsupported(reason) => panic!("unexpected unsupported: {reason:?}"),
    };
    let deepseek_receipt = detach_unhealthy_node_route(
        &mut node,
        "deepseek-8b",
        deepseek_route,
        |_| async { Ok(()) },
        |_| async { Ok(()) },
    )
    .await
    .unwrap();

    assert_eq!(
        node.workload_state("deepseek-8b"),
        Some(NodeState::Unhealthy)
    );
    assert!(begin_detached_route_recovery(&mut node, "deepseek-8b", qwen_receipt).is_err());
    assert_eq!(
        node.workload_state("deepseek-8b"),
        Some(NodeState::Unhealthy)
    );

    begin_detached_route_recovery(&mut node, "deepseek-8b", deepseek_receipt).unwrap();
    assert_eq!(
        node.workload_state("deepseek-8b"),
        Some(NodeState::Starting)
    );
}

#[tokio::test]
async fn failed_detach_can_retry_cleanup_without_requarantine() {
    let mut node = fake_orchestrator();

    let route = match prepare_and_attach_node_route(
        &mut node,
        ModelDemand::new("qwen-4b").unwrap(),
        |_, _| async { Ok(LocalRouteAttachmentId(601)) },
    )
    .await
    .unwrap()
    {
        LocalRouteOutcome::Routable(id) => id,
        LocalRouteOutcome::Unsupported(reason) => panic!("unexpected unsupported: {reason:?}"),
    };

    let first = detach_unhealthy_node_route(
        &mut node,
        "qwen-4b",
        route,
        |_| async { Ok(()) },
        |_| async { anyhow::bail!("cleanup failed") },
    )
    .await;
    assert!(first.is_err());
    assert_eq!(node.workload_state("qwen-4b"), Some(NodeState::Unhealthy));

    let receipt = detach_unhealthy_node_route(
        &mut node,
        "qwen-4b",
        route,
        |_| async { panic!("quarantine must not repeat") },
        |_| async { Ok(()) },
    )
    .await
    .unwrap();

    begin_detached_route_recovery(&mut node, "qwen-4b", receipt).unwrap();
    assert_eq!(node.workload_state("qwen-4b"), Some(NodeState::Starting));
}

#[tokio::test]
async fn stale_same_model_receipt_cannot_open_latest_recovery() {
    let mut node = fake_orchestrator();

    let first_route = match prepare_and_attach_node_route(
        &mut node,
        ModelDemand::new("qwen-4b").unwrap(),
        |_, _| async { Ok(LocalRouteAttachmentId(611)) },
    )
    .await
    .unwrap()
    {
        LocalRouteOutcome::Routable(id) => id,
        LocalRouteOutcome::Unsupported(reason) => panic!("unexpected unsupported: {reason:?}"),
    };
    let first_receipt = detach_unhealthy_node_route(
        &mut node,
        "qwen-4b",
        first_route,
        |_| async { Ok(()) },
        |_| async { Ok(()) },
    )
    .await
    .unwrap();
    let stale_receipt = first_receipt.clone();

    begin_detached_route_recovery(&mut node, "qwen-4b", first_receipt).unwrap();
    node.recover_until_ready("qwen-4b").await.unwrap();
    attach_ready_node_route(&mut node, "qwen-4b", || async {
        Ok(LocalRouteAttachmentId(612))
    })
    .await
    .unwrap();

    let current_receipt = detach_unhealthy_node_route(
        &mut node,
        "qwen-4b",
        LocalRouteAttachmentId(612),
        |_| async { Ok(()) },
        |_| async { Ok(()) },
    )
    .await
    .unwrap();

    assert!(begin_detached_route_recovery(&mut node, "qwen-4b", stale_receipt).is_err());
    assert_eq!(node.workload_state("qwen-4b"), Some(NodeState::Unhealthy));

    begin_detached_route_recovery(&mut node, "qwen-4b", current_receipt).unwrap();
    assert_eq!(node.workload_state("qwen-4b"), Some(NodeState::Starting));
}

#[tokio::test]
async fn failed_detach_cannot_enter_recovery_rail() {
    let mut node = fake_orchestrator();

    let route = match prepare_and_attach_node_route(
        &mut node,
        ModelDemand::new("qwen-4b").unwrap(),
        |_, _| async { Ok(LocalRouteAttachmentId(303)) },
    )
    .await
    .unwrap()
    {
        LocalRouteOutcome::Routable(id) => id,
        LocalRouteOutcome::Unsupported(reason) => panic!("unexpected unsupported: {reason:?}"),
    };

    let result = detach_unhealthy_node_route(
        &mut node,
        "qwen-4b",
        route,
        |_| async { Ok(()) },
        |_| async { anyhow::bail!("detach failed") },
    )
    .await;

    assert!(result.is_err());
    assert_eq!(node.workload_state("qwen-4b"), Some(NodeState::Unhealthy));
    assert_eq!(
        node.workload_attachment_id("qwen-4b"),
        Some(LocalRouteAttachmentId(303))
    );
}

#[tokio::test]
async fn injected_runtime_adapter_plan_flows_unchanged_to_every_consumer() {
    let plan = ProcessPlan {
        process: ProcessSpec {
            program: "/fake/adapter/runner".into(),
            args: vec!["fake-argument".into()],
        },
        readiness: ReadinessTarget {
            endpoint: "fake://runtime/ready".into(),
        },
        local_endpoint: "fake://runtime".into(),
    };
    let inputs = Arc::new(Mutex::new(None));
    let adapter = RecordingRuntimeAdapter {
        inputs: inputs.clone(),
        plan: plan.clone(),
    };
    let processes = RecordingProcessManager::default();
    let started = processes.started.clone();
    let readiness = RecordingReadinessProbe::default();
    let readiness_target = readiness.target.clone();
    let health = RecordingHealthProbe::default();
    let health_target = health.target.clone();
    let mut node = NodeOrchestrator::new(
        FakeModelResolver,
        adapter,
        NodeComposition::start(
            FakeHardwareProbe::default(),
            FakeArtifactPreparer,
            FakeRuntimePreparer,
            processes,
            readiness,
            health,
        ),
    );
    let attached = Arc::new(Mutex::new(None));
    let attached_by_call = attached.clone();

    let outcome = prepare_and_attach_node_route(
        &mut node,
        ModelDemand::new("qwen-4b").unwrap(),
        move |model, endpoint| {
            let attached = attached_by_call.clone();
            async move {
                *attached.lock().unwrap() = Some((model, endpoint));
                Ok(LocalRouteAttachmentId(574))
            }
        },
    )
    .await
    .unwrap();

    let (runtime, artifact) = inputs.lock().unwrap().clone().unwrap();
    assert!(!runtime.executable.is_empty());
    assert!(!artifact.local_path.is_empty());
    assert!(artifact.verified);
    assert_eq!(started.lock().unwrap().as_ref(), Some(&plan.process));
    assert_eq!(
        readiness_target.lock().unwrap().as_ref(),
        Some(&plan.readiness)
    );
    assert_eq!(
        health_target.lock().unwrap().as_ref(),
        Some(&plan.readiness)
    );
    assert_eq!(node.workload_plan("qwen-4b"), Some(&plan));
    assert_eq!(
        attached.lock().unwrap().clone(),
        Some(("qwen-4b".into(), plan.local_endpoint.clone()))
    );
    assert_eq!(
        outcome,
        LocalRouteOutcome::Routable(LocalRouteAttachmentId(574))
    );
    assert_eq!(node.workload_state("qwen-4b"), Some(NodeState::Routable));
}

#[tokio::test]
async fn runtime_adapter_failure_blocks_process_probes_and_attachment() {
    let adapter_calls = Arc::new(AtomicUsize::new(0));
    let adapter = FailingRuntimeAdapter {
        calls: adapter_calls.clone(),
    };
    let processes = RecordingProcessManager::default();
    let started = processes.started.clone();
    let readiness = RecordingReadinessProbe::default();
    let readiness_target = readiness.target.clone();
    let health = RecordingHealthProbe::default();
    let health_target = health.target.clone();
    let mut node = NodeOrchestrator::new(
        FakeModelResolver,
        adapter,
        NodeComposition::start(
            FakeHardwareProbe::default(),
            FakeArtifactPreparer,
            FakeRuntimePreparer,
            processes,
            readiness,
            health,
        ),
    );
    let attachment_calls = Arc::new(AtomicUsize::new(0));
    let calls_by_attach = attachment_calls.clone();

    let result = prepare_and_attach_node_route(
        &mut node,
        ModelDemand::new("qwen-4b").unwrap(),
        move |_, _| {
            calls_by_attach.fetch_add(1, Ordering::SeqCst);
            async { Ok(LocalRouteAttachmentId(999)) }
        },
    )
    .await;

    assert!(result.is_err());
    assert_eq!(adapter_calls.load(Ordering::SeqCst), 1);
    assert!(started.lock().unwrap().is_none());
    assert!(readiness_target.lock().unwrap().is_none());
    assert!(health_target.lock().unwrap().is_none());
    assert_eq!(attachment_calls.load(Ordering::SeqCst), 0);
    assert_eq!(node.workload_plan("qwen-4b"), None);
    assert_eq!(node.workload_state("qwen-4b"), Some(NodeState::Failed));
}

#[tokio::test]
async fn process_plan_receipt_is_persisted_before_process_start_completes() {
    let entered = Arc::new(AtomicBool::new(false));
    let mut node = NodeOrchestrator::new(
        FakeModelResolver,
        FakeRuntimeAdapter,
        NodeComposition::start(
            FakeHardwareProbe::default(),
            FakeArtifactPreparer,
            FakeRuntimePreparer,
            PendingProcessManager {
                entered: entered.clone(),
            },
            FakeReadinessProbe,
            FakeHealthProbe,
        ),
    );

    {
        let preparation = node.prepare_until_ready(ModelDemand::new("qwen-4b").unwrap());
        tokio::pin!(preparation);
        tokio::select! {
            result = &mut preparation => panic!("process start unexpectedly completed: {result:?}"),
            _ = async {
                while !entered.load(Ordering::SeqCst) {
                    tokio::task::yield_now().await;
                }
            } => {}
        }
    }

    assert!(entered.load(Ordering::SeqCst));
    assert!(node.workload_plan("qwen-4b").is_some());
    assert_eq!(node.workload_state("qwen-4b"), Some(NodeState::Starting));
}
