use burncloud_node_runtime::{
    FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe, FakeProcessManager,
    FakeReadinessProbe, FakeRuntimeAdapter, FakeRuntimePreparer, NodeComposition, NodeState,
};
use burncloud_router::local_attachment::LocalRouteAttachmentId;
use burncloud_server::node_attachment::{
    attach_ready_node_route, begin_detached_route_recovery, detach_unhealthy_node_route,
    prepare_and_attach_node_route, LocalRouteOutcome,
};
use burncloud_server::node_orchestrator::{ModelDemand, NodeOrchestrator};
use burncloud_server::node_request::NodeRequestState;
use burncloud_service_models::FakeModelResolver;

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
