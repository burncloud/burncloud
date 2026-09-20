use crate::node_attachment::{
    begin_detached_route_recovery, detach_unhealthy_node_route, prepare_and_attach_node_route,
    DetachedRoute, LocalRouteOutcome,
};
use crate::node_orchestrator::{ModelDemand, NodeOrchestrator};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use burncloud_node_runtime::{
    FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe, FakeProcessManager,
    FakeReadinessProbe, FakeRuntimeAdapter, FakeRuntimePreparer, NodeComposition, NodeState,
};
use burncloud_router::local_attachment::LocalRouteAttachmentId;
use burncloud_service_models::FakeModelResolver;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

type TestNodeOrchestrator = NodeOrchestrator<
    FakeModelResolver,
    FakeRuntimeAdapter,
    FakeHardwareProbe,
    FakeArtifactPreparer,
    FakeRuntimePreparer,
    FakeProcessManager,
    FakeReadinessProbe,
    FakeHealthProbe,
>;

#[derive(Debug, Default)]
struct EffectCounters {
    attach: AtomicU64,
    quarantine: AtomicU64,
    detach: AtomicU64,
}

impl EffectCounters {
    fn snapshot(&self) -> Value {
        json!({
            "attach": self.attach.load(Ordering::SeqCst),
            "quarantine": self.quarantine.load(Ordering::SeqCst),
            "detach": self.detach.load(Ordering::SeqCst),
        })
    }
}

struct TestHarness {
    orchestrator: TestNodeOrchestrator,
    known_models: BTreeSet<String>,
    receipt_history: Vec<DetachedRoute>,
    latest_detached: HashMap<String, LocalRouteAttachmentId>,
    effects: HashMap<String, Arc<EffectCounters>>,
    next_attachment: Arc<AtomicU64>,
}

impl TestHarness {
    fn new() -> Self {
        Self {
            orchestrator: NodeOrchestrator::new(
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
            ),
            known_models: BTreeSet::new(),
            receipt_history: Vec::new(),
            latest_detached: HashMap::new(),
            effects: HashMap::new(),
            next_attachment: Arc::new(AtomicU64::new(100)),
        }
    }

    fn effects_for(&mut self, model: &str) -> Arc<EffectCounters> {
        self.effects
            .entry(model.to_string())
            .or_insert_with(|| Arc::new(EffectCounters::default()))
            .clone()
    }

    fn response_for(&self, model: &str) -> Value {
        let state = self.orchestrator.workload_state(model);
        let active_attachment = self.orchestrator.workload_attachment_id(model);
        let detached_attachment = self.latest_detached.get(model).copied();
        let effects = self
            .effects
            .get(model)
            .map(|effects| effects.snapshot())
            .unwrap_or_else(|| json!({"attach": 0, "quarantine": 0, "detach": 0}));

        json!({
            "model": model,
            "state": state.map(|state| format!("{state:?}")),
            "attachment_id": active_attachment
                .or(detached_attachment)
                .map(|id| id.0),
            "detached": detached_attachment.is_some(),
            "effects": effects,
        })
    }

    fn receipt_by_attachment(
        &self,
        attachment_id: LocalRouteAttachmentId,
    ) -> Option<DetachedRoute> {
        self.receipt_history
            .iter()
            .rev()
            .find(|receipt| receipt.attachment_id() == attachment_id)
            .cloned()
    }
}

#[derive(Clone)]
pub struct NodeTestApiState {
    inner: Arc<Mutex<TestHarness>>,
}

impl Default for NodeTestApiState {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TestHarness::new())),
        }
    }
}

#[derive(Debug, Deserialize)]
struct DemandRequest {
    model: String,
}

#[derive(Debug, Default, Deserialize)]
struct DemandOptions {
    attach: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct UnhealthyOptions {
    quarantine: Option<String>,
    detach: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RecoveryOptions {
    attachment_id: Option<u64>,
}

type ApiResult = Result<Json<Value>, (StatusCode, Json<Value>)>;

fn api_error(
    status: StatusCode,
    code: &str,
    error: impl std::fmt::Display,
) -> (StatusCode, Json<Value>) {
    (
        status,
        Json(json!({
            "error": {
                "code": code,
                "message": error.to_string(),
            }
        })),
    )
}

/// The human black-box API is deliberately opt-in and disabled by default.
/// It is mounted on the existing BurnCloud router; this module never binds a socket.
pub fn enabled_from_env() -> bool {
    std::env::var("BURNCLOUD_NODE_TEST_API")
        .map(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true"))
        .unwrap_or(false)
}

pub fn router() -> Router {
    router_with_state(NodeTestApiState::default())
}

fn router_with_state(state: NodeTestApiState) -> Router {
    Router::new()
        .route("/test/node/models", get(list_models))
        .route("/test/node/demand", post(demand_model))
        .route("/test/node/models/{model}", get(model_status))
        .route("/test/node/models/{model}/unhealthy", post(mark_unhealthy))
        .route("/test/node/models/{model}/recover", post(recover_model))
        .with_state(state)
}

async fn list_models(State(state): State<NodeTestApiState>) -> ApiResult {
    let harness = state.inner.lock().await;
    let models: Vec<Value> = harness
        .known_models
        .iter()
        .map(|model| harness.response_for(model))
        .collect();
    Ok(Json(json!({ "models": models })))
}

async fn model_status(
    State(state): State<NodeTestApiState>,
    Path(model): Path<String>,
) -> ApiResult {
    let harness = state.inner.lock().await;
    if !harness.known_models.contains(&model) {
        return Err(api_error(
            StatusCode::NOT_FOUND,
            "node_model_not_found",
            format!("Node test workload '{model}' does not exist"),
        ));
    }
    Ok(Json(harness.response_for(&model)))
}

async fn demand_model(
    State(state): State<NodeTestApiState>,
    Query(options): Query<DemandOptions>,
    Json(request): Json<DemandRequest>,
) -> ApiResult {
    let demand = ModelDemand::new(request.model.clone())
        .map_err(|error| api_error(StatusCode::BAD_REQUEST, "invalid_model_demand", error))?;

    let mut harness = state.inner.lock().await;
    let model = demand.model.clone();
    harness.known_models.insert(model.clone());
    let effects = harness.effects_for(&model);
    let next_attachment = harness.next_attachment.clone();
    let fail_attach = options.attach.as_deref() == Some("fail");

    let outcome = prepare_and_attach_node_route(&mut harness.orchestrator, demand, move |_, _| {
        let effects = effects.clone();
        let next_attachment = next_attachment.clone();
        async move {
            effects.attach.fetch_add(1, Ordering::SeqCst);
            if fail_attach {
                anyhow::bail!("injected attach failure");
            }
            Ok(LocalRouteAttachmentId(
                next_attachment.fetch_add(1, Ordering::SeqCst),
            ))
        }
    })
    .await
    .map_err(|error| api_error(StatusCode::CONFLICT, "node_demand_failed", error))?;

    match outcome {
        LocalRouteOutcome::Routable(_) => Ok(Json(harness.response_for(&model))),
        LocalRouteOutcome::Unsupported(reason) => Ok(Json(json!({
            "model": model,
            "state": format!("{:?}", NodeState::LocalUnsupported),
            "unsupported": format!("{reason:?}"),
            "effects": harness.effects_for(&request.model).snapshot(),
        }))),
    }
}

async fn mark_unhealthy(
    State(state): State<NodeTestApiState>,
    Path(model): Path<String>,
    Query(options): Query<UnhealthyOptions>,
) -> ApiResult {
    let mut harness = state.inner.lock().await;
    let attachment_id = harness
        .orchestrator
        .workload_attachment_id(&model)
        .ok_or_else(|| {
            api_error(
                StatusCode::CONFLICT,
                "node_attachment_missing",
                format!("model '{model}' has no active attachment"),
            )
        })?;

    let effects = harness.effects_for(&model);
    let quarantine_effects = effects.clone();
    let detach_effects = effects.clone();
    let fail_quarantine = options.quarantine.as_deref() == Some("fail");
    let fail_detach = options.detach.as_deref() == Some("fail");

    let detached = detach_unhealthy_node_route(
        &mut harness.orchestrator,
        &model,
        attachment_id,
        move |_| async move {
            quarantine_effects.quarantine.fetch_add(1, Ordering::SeqCst);
            if fail_quarantine {
                anyhow::bail!("injected quarantine failure");
            }
            Ok(())
        },
        move |_| async move {
            detach_effects.detach.fetch_add(1, Ordering::SeqCst);
            if fail_detach {
                anyhow::bail!("injected detach failure");
            }
            Ok(())
        },
    )
    .await
    .map_err(|error| api_error(StatusCode::CONFLICT, "node_unhealthy_failed", error))?;

    harness
        .latest_detached
        .insert(model.clone(), detached.attachment_id());
    harness.receipt_history.push(detached);
    Ok(Json(harness.response_for(&model)))
}

async fn recover_model(
    State(state): State<NodeTestApiState>,
    Path(model): Path<String>,
    Query(options): Query<RecoveryOptions>,
) -> ApiResult {
    let mut harness = state.inner.lock().await;

    let requested_attachment = match options.attachment_id {
        Some(id) => LocalRouteAttachmentId(id),
        None => harness
            .latest_detached
            .get(&model)
            .copied()
            .ok_or_else(|| {
                api_error(
                    StatusCode::CONFLICT,
                    "detached_receipt_missing",
                    format!("model '{model}' has no latest detached route receipt"),
                )
            })?,
    };

    let receipt = harness
        .receipt_by_attachment(requested_attachment)
        .ok_or_else(|| {
            api_error(
                StatusCode::CONFLICT,
                "detached_receipt_unknown",
                format!(
                    "detached route receipt {} is not known to this test harness",
                    requested_attachment.0
                ),
            )
        })?;

    begin_detached_route_recovery(&mut harness.orchestrator, &model, receipt)
        .map_err(|error| api_error(StatusCode::CONFLICT, "node_recovery_rejected", error))?;

    if harness.latest_detached.get(&model) == Some(&requested_attachment) {
        harness.latest_detached.remove(&model);
    }

    harness
        .orchestrator
        .recover_until_ready(&model)
        .await
        .map_err(|error| api_error(StatusCode::CONFLICT, "node_recovery_failed", error))?;

    let effects = harness.effects_for(&model);
    let next_attachment = harness.next_attachment.clone();
    crate::node_attachment::attach_ready_node_route(&mut harness.orchestrator, &model, move || {
        let effects = effects.clone();
        let next_attachment = next_attachment.clone();
        async move {
            effects.attach.fetch_add(1, Ordering::SeqCst);
            Ok(LocalRouteAttachmentId(
                next_attachment.fetch_add(1, Ordering::SeqCst),
            ))
        }
    })
    .await
    .map_err(|error| api_error(StatusCode::CONFLICT, "node_reattach_failed", error))?;

    Ok(Json(harness.response_for(&model)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use tower::ServiceExt;

    async fn json_response(app: Router, request: Request<Body>) -> (StatusCode, Value) {
        let response = app.oneshot(request).await.unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice(&body).unwrap();
        (status, json)
    }

    fn demand_request(model: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/test/node/demand")
            .header("content-type", "application/json")
            .body(Body::from(json!({"model": model}).to_string()))
            .unwrap()
    }

    #[test]
    fn test_api_is_disabled_unless_explicitly_enabled() {
        assert!(!matches!(
            "".trim().to_ascii_lowercase().as_str(),
            "1" | "true"
        ));
    }

    #[tokio::test]
    async fn demand_and_repeated_demand_are_human_visible_and_idempotent() {
        let app = router();

        let (status, first) = json_response(app.clone(), demand_request("qwen-4b")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(first["state"], "Routable");
        assert_eq!(first["effects"]["attach"], 1);

        let first_attachment = first["attachment_id"].as_u64().unwrap();
        let (status, second) = json_response(app, demand_request("qwen-4b")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(second["attachment_id"], first_attachment);
        assert_eq!(second["effects"]["attach"], 1);
    }

    #[tokio::test]
    async fn two_models_keep_independent_blackbox_state() {
        let app = router();
        let _ = json_response(app.clone(), demand_request("qwen-4b")).await;
        let _ = json_response(app.clone(), demand_request("deepseek-8b")).await;

        let request = Request::builder()
            .uri("/test/node/models/qwen-4b")
            .body(Body::empty())
            .unwrap();
        let (status, qwen) = json_response(app.clone(), request).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(qwen["state"], "Routable");

        let request = Request::builder()
            .uri("/test/node/models/deepseek-8b")
            .body(Body::empty())
            .unwrap();
        let (status, deepseek) = json_response(app, request).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(deepseek["state"], "Routable");
        assert_ne!(qwen["attachment_id"], deepseek["attachment_id"]);
    }

    #[tokio::test]
    async fn unhealthy_then_recovery_is_observable_over_http() {
        let app = router();
        let (_, ready) = json_response(app.clone(), demand_request("qwen-4b")).await;
        let old_attachment = ready["attachment_id"].as_u64().unwrap();

        let request = Request::builder()
            .method("POST")
            .uri("/test/node/models/qwen-4b/unhealthy")
            .body(Body::empty())
            .unwrap();
        let (status, unhealthy) = json_response(app.clone(), request).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(unhealthy["state"], "Unhealthy");
        assert_eq!(unhealthy["detached"], true);
        assert_eq!(unhealthy["effects"]["quarantine"], 1);
        assert_eq!(unhealthy["effects"]["detach"], 1);

        let request = Request::builder()
            .method("POST")
            .uri("/test/node/models/qwen-4b/recover")
            .body(Body::empty())
            .unwrap();
        let (status, recovered) = json_response(app, request).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(recovered["state"], "Routable");
        assert_ne!(recovered["attachment_id"], old_attachment);
        assert_eq!(recovered["effects"]["attach"], 2);
    }

    #[tokio::test]
    async fn failed_detach_can_retry_without_second_quarantine() {
        let app = router();
        let _ = json_response(app.clone(), demand_request("qwen-4b")).await;

        let request = Request::builder()
            .method("POST")
            .uri("/test/node/models/qwen-4b/unhealthy?detach=fail")
            .body(Body::empty())
            .unwrap();
        let (status, _) = json_response(app.clone(), request).await;
        assert_eq!(status, StatusCode::CONFLICT);

        let request = Request::builder()
            .method("POST")
            .uri("/test/node/models/qwen-4b/unhealthy")
            .body(Body::empty())
            .unwrap();
        let (status, retried) = json_response(app, request).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(retried["effects"]["quarantine"], 1);
        assert_eq!(retried["effects"]["detach"], 2);
        assert_eq!(retried["state"], "Unhealthy");
    }

    #[tokio::test]
    async fn cross_model_receipt_is_rejected_by_framework() {
        let app = router();
        let (_, qwen) = json_response(app.clone(), demand_request("qwen-4b")).await;
        let qwen_attachment = qwen["attachment_id"].as_u64().unwrap();
        let _ = json_response(app.clone(), demand_request("deepseek-8b")).await;

        let request = Request::builder()
            .method("POST")
            .uri("/test/node/models/qwen-4b/unhealthy")
            .body(Body::empty())
            .unwrap();
        let _ = json_response(app.clone(), request).await;

        let request = Request::builder()
            .method("POST")
            .uri("/test/node/models/deepseek-8b/unhealthy")
            .body(Body::empty())
            .unwrap();
        let _ = json_response(app.clone(), request).await;

        let request = Request::builder()
            .method("POST")
            .uri(format!(
                "/test/node/models/deepseek-8b/recover?attachment_id={qwen_attachment}"
            ))
            .body(Body::empty())
            .unwrap();
        let (status, rejected) = json_response(app, request).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(rejected["error"]["code"], "node_recovery_rejected");
    }
}
