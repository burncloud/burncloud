use crate::local_attachment::{
    LocalRouteAttachment, LocalRouteAttachmentError, LocalRouteAttacher,
};
use async_trait::async_trait;
use burncloud_common::types::{Channel, ChannelType};
use burncloud_database::Database;
use burncloud_database_channel::ChannelProviderModel;

/// Production adapter that makes a READY local endpoint visible through the
/// existing BurnCloud channel/ability routing truth.
///
/// It deliberately reuses `ChannelProviderModel`; it does not create a second
/// ModelRouter or a second routing table.
pub struct ExistingRouterLocalAttacher {
    db: Database,
}

impl ExistingRouterLocalAttacher {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    fn build_channel(attachment: &LocalRouteAttachment) -> Channel {
        Channel {
            id: 0,
            type_: ChannelType::OpenAI as i32,
            key: String::new(),
            status: 1,
            name: format!("BurnCloud Node: {}", attachment.model),
            weight: 1,
            created_time: None,
            test_time: None,
            response_time: None,
            base_url: Some(attachment.base_url.clone()),
            models: attachment.model.clone(),
            group: "default".to_string(),
            used_quota: 0,
            model_mapping: None,
            priority: 0,
            auto_ban: 1,
            other_info: None,
            tag: Some("burncloud-node-local".to_string()),
            setting: None,
            param_override: None,
            header_override: None,
            remark: Some("Managed by BurnCloud Node runtime".to_string()),
            api_version: Some("default".to_string()),
            pricing_region: None,
            rpm_cap: None,
            tpm_cap: None,
            reservation_green: None,
            reservation_yellow: None,
            reservation_red: None,
        }
    }
}

#[async_trait]
impl LocalRouteAttacher for ExistingRouterLocalAttacher {
    async fn attach(
        &self,
        attachment: LocalRouteAttachment,
    ) -> Result<(), LocalRouteAttachmentError> {
        let mut channel = Self::build_channel(&attachment);
        ChannelProviderModel::create(&self.db, &mut channel)
            .await
            .map_err(|error| LocalRouteAttachmentError::AttachFailed(error.to_string()))?;
        Ok(())
    }

    async fn detach(&self, _model: &str) -> Result<(), LocalRouteAttachmentError> {
        // Detach needs a stable ownership identity, not a model-name delete.
        // Keep this fail-closed until the contract carries the created channel id.
        Err(LocalRouteAttachmentError::DetachFailed(
            "detach requires stable local channel identity".to_string(),
        ))
    }
}
