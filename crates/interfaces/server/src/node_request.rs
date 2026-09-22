use axum::body::Body;
use axum::http::{Response, StatusCode};
use burncloud_node_runtime::NodeState;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// What the synchronous request path should do after checking the existing
/// ModelRouter for the requested model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRequestDisposition {
    /// Existing BurnCloud routing truth already has a candidate. Serve it now;
    /// the candidate may be an external Provider or a READY local Node channel.
    /// Candidate preference remains owned by Traffic/Scheduler, never Node.
    UseExistingRoute,
    /// No existing candidate can serve the request, but Node is actively
    /// converging toward a local route.
    ModelPreparing(NodeState),
    /// No existing candidate exists and Node is not actively preparing one.
    Unavailable,
}

/// Application-owned view of per-model Node progress used only after a true
/// existing-router miss. It stores status, not routing preference.
#[derive(Debug, Clone, Default)]
pub struct NodeRequestState {
    states: Arc<RwLock<HashMap<String, NodeState>>>,
}

impl NodeRequestState {
    pub fn publish(&self, model: &str, state: NodeState) {
        if let Ok(mut states) = self.states.write() {
            states.insert(model.to_string(), state);
        }
    }

    pub fn state_for(&self, model: &str) -> Option<NodeState> {
        self.states.read().ok()?.get(model).copied()
    }

    pub fn route_miss_response(&self, model: &str) -> Option<Response<Body>> {
        let state = self.state_for(model)?;
        match node_request_disposition(false, state) {
            NodeRequestDisposition::ModelPreparing(state) => {
                Some(model_preparing_response(model, state))
            }
            NodeRequestDisposition::UseExistingRoute | NodeRequestDisposition::Unavailable => None,
        }
    }
}

/// Decide the request result without changing routing truth.
///
/// Existing ModelRouter candidates always win. Node state is consulted only
/// when the existing ModelRouter has no candidate for the model. This layer
/// must not decide local-vs-external preference; that belongs to Traffic.
pub const fn node_request_disposition(
    has_existing_candidate: bool,
    node_state: NodeState,
) -> NodeRequestDisposition {
    if has_existing_candidate {
        return NodeRequestDisposition::UseExistingRoute;
    }

    match node_state {
        NodeState::Resolving
        | NodeState::PreparingArtifact
        | NodeState::ArtifactReady
        | NodeState::PreparingRuntime
        | NodeState::Starting
        | NodeState::WaitingReady
        | NodeState::Ready => NodeRequestDisposition::ModelPreparing(node_state),
        NodeState::Absent
        | NodeState::LocalUnsupported
        | NodeState::Routable
        | NodeState::Failed
        | NodeState::Unhealthy => NodeRequestDisposition::Unavailable,
    }
}

/// Stable client contract for the "no route yet, local model is preparing"
/// case. The request returns immediately instead of waiting for download/startup.
pub fn model_preparing_response(model: &str, state: NodeState) -> Response<Body> {
    let body = serde_json::json!({
        "error": {
            "message": format!("Model '{model}' is being prepared locally"),
            "type": "service_unavailable",
            "code": "MODEL_PREPARING",
            "state": format!("{state:?}"),
        }
    });

    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header("content-type", "application/json")
        .header("Retry-After", "5")
        .body(Body::from(body.to_string()))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_route_always_serves_even_while_local_model_is_preparing() {
        for state in [
            NodeState::Resolving,
            NodeState::PreparingArtifact,
            NodeState::PreparingRuntime,
            NodeState::Starting,
            NodeState::WaitingReady,
            NodeState::Routable,
        ] {
            assert_eq!(
                node_request_disposition(true, state),
                NodeRequestDisposition::UseExistingRoute
            );
        }
    }

    #[test]
    fn missing_route_reports_model_preparing_for_every_active_prepare_state() {
        for state in [
            NodeState::Resolving,
            NodeState::PreparingArtifact,
            NodeState::ArtifactReady,
            NodeState::PreparingRuntime,
            NodeState::Starting,
            NodeState::WaitingReady,
            NodeState::Ready,
        ] {
            assert_eq!(
                node_request_disposition(false, state),
                NodeRequestDisposition::ModelPreparing(state)
            );
        }
    }

    #[test]
    fn missing_route_is_not_called_preparing_when_node_is_not_converging() {
        for state in [
            NodeState::Absent,
            NodeState::LocalUnsupported,
            NodeState::Failed,
            NodeState::Unhealthy,
            NodeState::Routable,
        ] {
            assert_eq!(
                node_request_disposition(false, state),
                NodeRequestDisposition::Unavailable
            );
        }
    }

    #[test]
    fn model_preparing_response_has_stable_retryable_contract() {
        let response = model_preparing_response("qwen-4b", NodeState::PreparingArtifact);
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers().get("Retry-After").unwrap(), "5");
    }

    #[test]
    fn request_state_answers_only_for_the_requested_preparing_model() {
        let state = NodeRequestState::default();
        state.publish("qwen-4b", NodeState::PreparingArtifact);
        state.publish("other-model", NodeState::Failed);

        assert!(state.route_miss_response("qwen-4b").is_some());
        assert!(state.route_miss_response("other-model").is_none());
        assert!(state.route_miss_response("unknown-model").is_none());
    }
}
