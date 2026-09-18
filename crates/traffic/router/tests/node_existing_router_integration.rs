#[path = "../src/local_attachment.rs"]
mod local_attachment;

use burncloud_database::create_database_with_url;
use burncloud_router::model_router::ModelRouter;
use local_attachment::{ExistingRouterLocalAttacher, LocalRouteAttacher, LocalRouteAttachment};
use std::sync::Arc;

/// Phase-0 S0-08 acceptance: a READY local Node endpoint becomes discoverable
/// through BurnCloud's existing channel/ability routing truth, and detaching
/// the exact created channel removes it again.
#[tokio::test]
async fn ready_node_endpoint_round_trips_through_existing_model_router() {
    let temp = tempfile::NamedTempFile::new().expect("temp database");
    let url = format!("sqlite://{}?mode=rwc", temp.path().display());
    let db = Arc::new(
        create_database_with_url(&url)
            .await
            .expect("initialize migrated database"),
    );

    let attacher = ExistingRouterLocalAttacher::new(db.clone());
    let model_router = ModelRouter::new(db);
    let model = "phase0-local-model";

    assert!(model_router
        .get_candidates("default", model)
        .await
        .expect("query before attach")
        .is_empty());

    let attachment_id = attacher
        .attach(LocalRouteAttachment {
            model: model.to_string(),
            base_url: "http://127.0.0.1:18080".to_string(),
        })
        .await
        .expect("attach ready local endpoint");

    let candidates = model_router
        .get_candidates("default", model)
        .await
        .expect("existing ModelRouter must discover local channel");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].0.id, attachment_id.0);
    assert_eq!(
        candidates[0].0.base_url.as_deref(),
        Some("http://127.0.0.1:18080")
    );

    attacher
        .detach(attachment_id)
        .await
        .expect("detach exact local channel");

    assert!(model_router
        .get_candidates("default", model)
        .await
        .expect("query after detach")
        .is_empty());
}
