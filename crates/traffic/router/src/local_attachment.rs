use async_trait::async_trait;

#[path = "local_attachment_adapter.rs"]
mod adapter;
pub use adapter::ExistingRouterLocalAttacher;

/// A local inference capability that has already passed runtime readiness.
///
/// Traffic owns the contract for making that capability visible to the existing
/// routing path. Node/runtime code must not create or own a second ModelRouter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRouteAttachment {
    pub model: String,
    pub base_url: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LocalRouteAttachmentError {
    #[error("local route attachment failed: {0}")]
    AttachFailed(String),
    #[error("local route detachment failed: {0}")]
    DetachFailed(String),
}

/// Traffic-owned port for attaching a ready local capability to BurnCloud's
/// existing routing truth.
#[async_trait]
pub trait LocalRouteAttacher: Send + Sync {
    async fn attach(
        &self,
        attachment: LocalRouteAttachment,
    ) -> Result<(), LocalRouteAttachmentError>;

    async fn detach(&self, model: &str) -> Result<(), LocalRouteAttachmentError>;
}
