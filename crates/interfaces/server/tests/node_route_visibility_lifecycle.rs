#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Integration-test preconditions use fail-fast assertions so lifecycle regressions point at the exact step that failed."
)]
//! Regression oracle for #649.
//!
//! The retired inference integration test used the removed `router_upstreams` table as its oracle. That is no
//! longer routing truth. A READY local Node endpoint is now published through `ExistingRouterLocalAttacher`,
//! which writes the ordinary `channel_providers` / `channel_abilities` truth consumed by `ModelRouter`.
//!
//! This test composes the current Node preparation seam with that real Traffic adapter. The runtime side uses
//! BurnCloud's fake resolver/probes/process manager, so it does not start a real inference engine or require a
//! model path. The route side uses a real migrated SQLite database, because visibility through ModelRouter is
//! the behaviour being proved.

use burncloud_database::create_database_with_url;
use burncloud_node_runtime::{
    FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe, FakeProcessManager, FakeReadinessProbe,
    FakeRuntimeAdapter, FakeRuntimePreparer, NodeComposition, NodeState,
};
use burncloud_router::local_attachment::{
    ExistingRouterLocalAttacher, LocalRouteAttacher, LocalRouteAttachment, LocalRouteAttachmentId,
};
use burncloud_router::model_router::ModelRouter;
use burncloud_server::node_attachment::{
    detach_unhealthy_node_route, prepare_and_attach_node_route, LocalRouteOutcome,
};
use burncloud_server::node_orchestrator::{ModelDemand, NodeOrchestrator};
use burncloud_service_models::FakeModelResolver;
use std::sync::Arc;

fn sqlite_url(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let absolute = normalized.starts_with('/') || {
        let head: Vec<char> = normalized.chars().take(3).collect();
        head.len() == 3 && head[0].is_ascii_alphabetic() && head[1] == ':' && head[2] == '/'
    };
    if absolute {
        format!("sqlite:///{}?mode=rwc", normalized.trim_start_matches('/'))
    } else {
        format!("sqlite://{}?mode=rwc", normalized)
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

async fn assert_visible(router: &ModelRouter, model: &str, expected: LocalRouteAttachmentId) {
    let candidates = router
        .get_candidates("default", model)
        .await
        .expect("query current routing truth");
    assert_eq!(
        candidates.len(),
        1,
        "READY Node endpoint must be discoverable through the existing ModelRouter"
    );
    assert_eq!(candidates[0].0.id, expected.0);
}

async fn assert_not_visible(router: &ModelRouter, model: &str) {
    let candidates = router
        .get_candidates("default", model)
        .await
        .expect("query current routing truth");
    assert!(
        candidates.is_empty(),
        "route still visible after stop/detach; cleanup oracle caught a leaked local attachment"
    );
}

async fn prepared_route(
    model: &str,
) -> (
    tempfile::NamedTempFile,
    Arc<burncloud_database::Database>,
    Arc<ExistingRouterLocalAttacher>,
    ModelRouter,
    NodeOrchestrator<
        FakeModelResolver,
        FakeRuntimeAdapter,
        FakeHardwareProbe,
        FakeArtifactPreparer,
        FakeRuntimePreparer,
        FakeProcessManager,
        FakeReadinessProbe,
        FakeHealthProbe,
    >,
    LocalRouteAttachmentId,
) {
    let temp = tempfile::NamedTempFile::new().expect("temporary route database");
    let url = sqlite_url(&temp.path().to_string_lossy());
    let db = Arc::new(
        create_database_with_url(&url)
            .await
            .expect("initialize migrated test database"),
    );
    let attacher = Arc::new(ExistingRouterLocalAttacher::new(db.clone()));
    let router = ModelRouter::new(db.clone());
    let mut orchestrator = fake_orchestrator();

    let attach_side_effect = attacher.clone();
    let outcome = prepare_and_attach_node_route(
        &mut orchestrator,
        ModelDemand::new(model).expect("valid model demand"),
        move |model, base_url| {
            let attacher = attach_side_effect.clone();
            async move {
                attacher
                    .attach(LocalRouteAttachment { model, base_url })
                    .await
                    .map_err(|error| anyhow::anyhow!(error.to_string()))
            }
        },
    )
    .await
    .expect("fake Node preparation and real route attachment must succeed");

    let attachment_id = match outcome {
        LocalRouteOutcome::Routable(id) => id,
        LocalRouteOutcome::Unsupported(reason) => {
            panic!("fake resolver unexpectedly rejected {model}: {reason:?}")
        }
    };

    (temp, db, attacher, router, orchestrator, attachment_id)
}

#[tokio::test]
async fn node_start_makes_route_visible_and_stop_detach_removes_it() {
    let model = "issue-649-lifecycle-model";
    let (_temp, db, attacher, router, mut orchestrator, attachment_id) = prepared_route(model).await;

    // Start oracle: READY is not enough by itself; Traffic must be able to discover the published route.
    assert_visible(&router, model, attachment_id).await;
    assert_eq!(
        orchestrator.workload_attachment_id(model),
        Some(attachment_id),
        "Node lifecycle must retain the exact Traffic attachment identity"
    );

    // Current teardown lifecycle: Traffic is quarantined first (fail closed), then the exact attachment is
    // deleted, and only then does node_attachment clear the lifecycle receipt. This is the public application
    // seam; the test does not widen NodeOrchestrator internals just to manufacture an oracle.
    let quarantine_attacher = attacher.clone();
    let detach_attacher = attacher.clone();
    detach_unhealthy_node_route(
        &mut orchestrator,
        model,
        attachment_id,
        move |id| {
            let attacher = quarantine_attacher.clone();
            async move {
                attacher
                    .quarantine(id)
                    .await
                    .map_err(|error| anyhow::anyhow!(error.to_string()))
            }
        },
        move |id| {
            let attacher = detach_attacher.clone();
            async move {
                attacher
                    .detach(id)
                    .await
                    .map_err(|error| anyhow::anyhow!(error.to_string()))
            }
        },
    )
    .await
    .expect("quarantine and detach exact local route");

    assert_not_visible(&router, model).await;
    assert_eq!(orchestrator.workload_attachment_id(model), None);
    assert_eq!(orchestrator.workload_state(model), Some(NodeState::Unhealthy));

    db.close().await.expect("close lifecycle test database");
}

/// Error-detection evidence required by #649: this deliberately injects the old failure mode by omitting the
/// stop/detach side effect. The same absence oracle used above must panic while the local route is still visible.
/// `should_panic` turns that expected detection into a passing mutation-guard test rather than committing a
/// permanently failing test.
#[tokio::test]
#[should_panic(expected = "route still visible after stop/detach")]
async fn lifecycle_oracle_detects_a_stop_that_forgets_route_cleanup() {
    let model = "issue-649-missing-cleanup-mutation";
    let (_temp, _db, _attacher, router, _orchestrator, attachment_id) = prepared_route(model).await;

    assert_visible(&router, model, attachment_id).await;

    // Injected mutation: the stop path forgot the Traffic quarantine/detach cleanup entirely.
    assert_not_visible(&router, model).await;
}
