#[path = "../src/local_attachment.rs"]
mod local_attachment;

use async_trait::async_trait;
use local_attachment::{LocalRouteAttachment, LocalRouteAttachmentError, LocalRouteAttacher};
use std::sync::Mutex;

#[derive(Default)]
struct FakeExistingRouterAttachment {
    attached: Mutex<Vec<LocalRouteAttachment>>,
}

#[async_trait]
impl LocalRouteAttacher for FakeExistingRouterAttachment {
    async fn attach(
        &self,
        attachment: LocalRouteAttachment,
    ) -> Result<(), LocalRouteAttachmentError> {
        self.attached.lock().unwrap().push(attachment);
        Ok(())
    }

    async fn detach(&self, model: &str) -> Result<(), LocalRouteAttachmentError> {
        self.attached.lock().unwrap().retain(|entry| entry.model != model);
        Ok(())
    }
}

#[tokio::test]
async fn traffic_owns_attachment_of_ready_local_capability() {
    let router = FakeExistingRouterAttachment::default();
    let attachment = LocalRouteAttachment {
        model: "fake-model".into(),
        base_url: "http://127.0.0.1:18080".into(),
    };

    router.attach(attachment.clone()).await.unwrap();
    assert_eq!(router.attached.lock().unwrap().as_slice(), &[attachment]);

    router.detach("fake-model").await.unwrap();
    assert!(router.attached.lock().unwrap().is_empty());
}
