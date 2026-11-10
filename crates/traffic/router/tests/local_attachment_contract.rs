#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file. The fake attacher stores through a `Mutex`, so recording an attachment \
              requires a lock acquisition whose failure the test cannot act on, and the assertions are \
              the test. The sibling test files in this directory carry the same allowance; this one \
              was missing it."
)]

use async_trait::async_trait;
use burncloud_router::local_attachment::{
    LocalRouteAttacher, LocalRouteAttachment, LocalRouteAttachmentError, LocalRouteAttachmentId,
};
use std::sync::Mutex;

#[derive(Default)]
struct FakeExistingRouterAttachment {
    attached: Mutex<Vec<(LocalRouteAttachmentId, LocalRouteAttachment)>>,
}

#[async_trait]
impl LocalRouteAttacher for FakeExistingRouterAttachment {
    async fn attach(
        &self,
        attachment: LocalRouteAttachment,
    ) -> Result<LocalRouteAttachmentId, LocalRouteAttachmentError> {
        let id = LocalRouteAttachmentId(42);
        self.attached.lock().unwrap().push((id, attachment));
        Ok(id)
    }

    async fn quarantine(
        &self,
        _attachment_id: LocalRouteAttachmentId,
    ) -> Result<(), LocalRouteAttachmentError> {
        Ok(())
    }

    async fn detach(
        &self,
        attachment_id: LocalRouteAttachmentId,
    ) -> Result<(), LocalRouteAttachmentError> {
        self.attached
            .lock()
            .unwrap()
            .retain(|(id, _)| *id != attachment_id);
        Ok(())
    }
}

#[tokio::test]
async fn traffic_owns_attachment_of_ready_local_capability_by_stable_identity() {
    let router = FakeExistingRouterAttachment::default();
    let attachment = LocalRouteAttachment {
        model: "fake-model".into(),
        base_url: "http://127.0.0.1:18080".into(),
    };

    let attachment_id = router.attach(attachment.clone()).await.unwrap();
    assert_eq!(attachment_id, LocalRouteAttachmentId(42));
    assert_eq!(
        router.attached.lock().unwrap().as_slice(),
        &[(attachment_id, attachment)]
    );

    router.detach(attachment_id).await.unwrap();
    assert!(router.attached.lock().unwrap().is_empty());
}

// 以下测试使用生产适配器及真实迁移数据库，示例 42 不用作实现验收。
#[tokio::test]
async fn production_input_boundary_and_exact_model_capability() {
    use burncloud_router::local_attachment::ExistingRouterLocalAttacher;
    use burncloud_supply_channel::ChannelService;
    let file = tempfile::NamedTempFile::new().unwrap();
    let url = burncloud_database::sqlite_url(&file.path().to_string_lossy(), cfg!(windows), true);
    let db = std::sync::Arc::new(
        burncloud_database::create_database_with_url(&url)
            .await
            .unwrap(),
    );
    let attacher = ExistingRouterLocalAttacher::new(db.clone());
    for model in [
        "".to_string(),
        " test".into(),
        "test\u{3000}".into(),
        "a,b".into(),
        "a\nb".into(),
        "a\u{7f}b".into(),
        "界".repeat(35),
    ] {
        assert!(matches!(
            attacher
                .attach(LocalRouteAttachment {
                    model,
                    base_url: "http://127.0.0.1".into()
                })
                .await,
            Err(LocalRouteAttachmentError::AttachFailed(_))
        ));
    }
    for base_url in [
        "",
        "ftp://127.0.0.1",
        "http://localhost",
        "http://example.com",
        "http://192.168.1.2",
        "http://[::2]",
        "http://127.0.0.1/v1",
        "http://127.0.0.1/./",
        "http://127.0.0.1/%2e/",
        "http://user@127.0.0.1",
        "http://@127.0.0.1",
        "http://127.0.0.1?",
        "http://127.0.0.1#",
        " http://127.0.0.1",
        "http://127.0.0.1\n",
        "http://127.0.0.1//",
        "http://127.0.0.1:99999",
        "http://127.0.0.1\\",
    ] {
        assert!(
            matches!(
                attacher
                    .attach(LocalRouteAttachment {
                        model: "test".into(),
                        base_url: base_url.into()
                    })
                    .await,
                Err(LocalRouteAttachmentError::AttachFailed(_))
            ),
            "{base_url:?}"
        );
    }
    for id in [0, -1, i32::MIN] {
        assert!(matches!(
            attacher.quarantine(LocalRouteAttachmentId(id)).await,
            Err(LocalRouteAttachmentError::DetachFailed(_))
        ));
        assert!(matches!(
            attacher.detach(LocalRouteAttachmentId(id)).await,
            Err(LocalRouteAttachmentError::DetachFailed(_))
        ));
    }
    assert!(ChannelService::list(&db, i32::MAX, 0)
        .await
        .unwrap()
        .is_empty());
    for model in ["Qwen:中文 4b".to_string(), "界".repeat(34)] {
        let server = probe_server(&model).await;
        let normalized = server.0.as_str();
        let base_url = format!("{normalized}/");
        let id = attacher
            .attach(LocalRouteAttachment {
                model: model.clone(),
                base_url,
            })
            .await
            .unwrap();
        let channel = ChannelService::get_by_id(&db, id.0).await.unwrap().unwrap();
        assert_eq!(channel.models, model);
        assert_eq!(channel.base_url.as_deref(), Some(normalized));
        let router = burncloud_router::model_router::ModelRouter::new(db.clone());
        assert_eq!(
            router
                .get_candidates("default", &model)
                .await
                .unwrap()
                .len(),
            1
        );
        attacher.detach(id).await.unwrap();
        server.1.abort();
        assert!(server.1.await.unwrap_err().is_cancelled());
    }
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn non_local_id_is_never_quarantined_or_deleted() {
    use burncloud_router::local_attachment::ExistingRouterLocalAttacher;
    use burncloud_supply_channel::ChannelService;
    let file = tempfile::NamedTempFile::new().unwrap();
    let url = burncloud_database::sqlite_url(&file.path().to_string_lossy(), cfg!(windows), true);
    let db = std::sync::Arc::new(
        burncloud_database::create_database_with_url(&url)
            .await
            .unwrap(),
    );
    let attacher = ExistingRouterLocalAttacher::new(db.clone());
    let server = probe_server("test").await;
    let id = attacher
        .attach(LocalRouteAttachment {
            model: "test".into(),
            base_url: server.0.clone(),
        })
        .await
        .unwrap();
    let mut channel = ChannelService::get_by_id(&db, id.0).await.unwrap().unwrap();
    channel.tag = Some("not-local".into());
    ChannelService::update(&db, &channel).await.unwrap();
    assert!(attacher.quarantine(id).await.is_err());
    assert!(attacher.detach(id).await.is_err());
    assert_eq!(
        ChannelService::get_by_id(&db, id.0)
            .await
            .unwrap()
            .unwrap()
            .status,
        1
    );
    assert_eq!(
        ChannelService::list_enabled_at_highest_priority(&db, "default", "test")
            .await
            .unwrap()
            .len(),
        1
    );
    server.1.abort();
    assert!(server.1.await.unwrap_err().is_cancelled());
    db.get_connection().unwrap().pool().close().await;
}

async fn probe_server(model: &str) -> (String, tokio::task::JoinHandle<()>) {
    let model = model.to_owned();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let app = axum::Router::new().route(
        "/v1/models",
        axum::routing::get(move || {
            let model = model.clone();
            async move { axum::Json(serde_json::json!({"data":[{"id":model}]})) }
        }),
    );
    (
        url,
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        }),
    )
}
