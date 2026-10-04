use burncloud_database::create_database_with_url;

/// Build a SQLite connection URL for a filesystem path.
///
/// **The three-slash rule.** `sqlite:///C:/data/db.sqlite` is the absolute form; `sqlite://C:/data/db.sqlite`
/// has only two slashes after the scheme, so SQLite reads `C:` as a **host** and refuses the connection with
/// `(code: 14) unable to open database file`. A relative path keeps two slashes, because with three the leading
/// `/` would become part of the path and turn `data/db.sqlite` into `/data/db.sqlite`.
///
/// A local copy of `burncloud_database::sqlite_url`, which lives in an open pull request rather than on `main`;
/// see the note in this crate's `tests/common.rs`.
fn sqlite_url(path: &str) -> String {
    let normalised = path.replace('\\', "/");
    let absolute = normalised.starts_with('/') || {
        let head: Vec<char> = normalised.chars().take(3).collect();
        head.len() == 3 && head[0].is_ascii_alphabetic() && head[1] == ':' && head[2] == '/'
    };
    if absolute {
        format!("sqlite:///{}?mode=rwc", normalised.trim_start_matches('/'))
    } else {
        format!("sqlite://{}?mode=rwc", normalised)
    }
}

use burncloud_router::local_attachment::{
    ExistingRouterLocalAttacher, LocalRouteAttacher, LocalRouteAttachment,
};
use burncloud_router::model_router::ModelRouter;
use std::sync::Arc;

/// Phase-0 S0-08 acceptance: a READY local Node endpoint becomes discoverable
/// through BurnCloud's existing channel/ability routing truth, and detaching
/// the exact created channel removes it again.
#[tokio::test]
async fn ready_node_endpoint_round_trips_through_existing_model_router() {
    let temp = tempfile::NamedTempFile::new().expect("temp database");
    let url = sqlite_url(&temp.path().to_string_lossy());
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
        .quarantine(attachment_id)
        .await
        .expect("quarantine unhealthy local channel");

    assert!(model_router
        .get_candidates("default", model)
        .await
        .expect("query after quarantine")
        .is_empty());

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

/// #564 P0-A: quarantine is one Traffic transaction. If disabling the channel
/// cannot commit, channel_abilities must roll back too; Node may then keep the
/// lifecycle Routable instead of believing a half-quarantine succeeded.
#[tokio::test]
async fn failed_quarantine_rolls_back_routing_ability() {
    let temp = tempfile::NamedTempFile::new().expect("temp database");
    let url = sqlite_url(&temp.path().to_string_lossy());
    let db = Arc::new(
        create_database_with_url(&url)
            .await
            .expect("initialize migrated database"),
    );

    let attacher = ExistingRouterLocalAttacher::new(db.clone());
    let model_router = ModelRouter::new(db.clone());
    let model = "phase0-quarantine-rollback-model";

    let attachment_id = attacher
        .attach(LocalRouteAttachment {
            model: model.to_string(),
            base_url: "http://127.0.0.1:18081".to_string(),
        })
        .await
        .expect("attach ready local endpoint");

    assert_eq!(
        model_router
            .get_candidates("default", model)
            .await
            .expect("candidate before quarantine")
            .len(),
        1
    );

    let conn = db.get_connection().expect("database connection");
    burncloud_database::sqlx::query(
        r#"
        CREATE TRIGGER fail_node_quarantine
        BEFORE UPDATE OF status ON channel_providers
        WHEN NEW.id = OLD.id AND NEW.status = 3
        BEGIN
            SELECT RAISE(ABORT, 'forced quarantine failure');
        END
        "#,
    )
    .execute(conn.pool())
    .await
    .expect("install failure trigger");

    assert!(attacher.quarantine(attachment_id).await.is_err());

    // DELETE channel_abilities happened earlier in the same transaction. The
    // failed status update must roll it back, otherwise this would be empty.
    assert_eq!(
        model_router
            .get_candidates("default", model)
            .await
            .expect("candidate after failed quarantine")
            .len(),
        1
    );

    burncloud_database::sqlx::query("DROP TRIGGER fail_node_quarantine")
        .execute(conn.pool())
        .await
        .expect("remove failure trigger");

    attacher
        .quarantine(attachment_id)
        .await
        .expect("quarantine succeeds after trigger removal");
    assert!(model_router
        .get_candidates("default", model)
        .await
        .expect("candidate after successful quarantine")
        .is_empty());
}
