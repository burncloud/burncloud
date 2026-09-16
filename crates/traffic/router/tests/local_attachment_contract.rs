#[path = "../src/local_attachment.rs"]
mod local_attachment;

use async_trait::async_trait;
use local_attachment::{
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
