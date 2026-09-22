use crate::local_attachment::{
    LocalRouteAttacher, LocalRouteAttachment, LocalRouteAttachmentError, LocalRouteAttachmentId,
};
use async_trait::async_trait;
use burncloud_common::types::{Channel, ChannelType};
use burncloud_database::{adapt_sql, sqlx, Database};
use burncloud_database_channel::ChannelProviderModel;
use std::sync::Arc;

/// Production adapter that makes a READY local endpoint visible through the
/// existing BurnCloud channel/ability routing truth.
///
/// It deliberately reuses `ChannelProviderModel`; it does not create a second
/// ModelRouter or a second routing table.
pub struct ExistingRouterLocalAttacher {
    db: Arc<Database>,
}

impl ExistingRouterLocalAttacher {
    pub fn new(db: Arc<Database>) -> Self {
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
    ) -> Result<LocalRouteAttachmentId, LocalRouteAttachmentError> {
        let mut channel = Self::build_channel(&attachment);
        let channel_id = ChannelProviderModel::create(self.db.as_ref(), &mut channel)
            .await
            .map_err(|error| LocalRouteAttachmentError::AttachFailed(error.to_string()))?;
        Ok(LocalRouteAttachmentId(channel_id))
    }

    async fn quarantine(
        &self,
        attachment_id: LocalRouteAttachmentId,
    ) -> Result<(), LocalRouteAttachmentError> {
        let conn = self
            .db
            .get_connection()
            .map_err(|error| LocalRouteAttachmentError::DetachFailed(error.to_string()))?;
        let pool = conn.pool();
        let is_postgres = self.db.kind() == "postgres";

        // Fail-closed must be atomic from Traffic's point of view:
        // either both the routing ability and channel status change, or neither
        // does. This prevents a half-quarantined channel from remaining
        // discoverable through channel_abilities.
        let mut tx = pool
            .begin()
            .await
            .map_err(|error| LocalRouteAttachmentError::DetachFailed(error.to_string()))?;

        let delete_abilities = adapt_sql(
            is_postgres,
            "DELETE FROM channel_abilities WHERE channel_id = ?",
        );
        sqlx::query(&delete_abilities)
            .bind(attachment_id.0)
            .execute(&mut *tx)
            .await
            .map_err(|error| LocalRouteAttachmentError::DetachFailed(error.to_string()))?;

        let disable_channel = adapt_sql(
            is_postgres,
            "UPDATE channel_providers SET status = 3 WHERE id = ?",
        );
        let result = sqlx::query(&disable_channel)
            .bind(attachment_id.0)
            .execute(&mut *tx)
            .await
            .map_err(|error| LocalRouteAttachmentError::DetachFailed(error.to_string()))?;

        if result.rows_affected() != 1 {
            return Err(LocalRouteAttachmentError::DetachFailed(format!(
                "local route attachment {} no longer exists",
                attachment_id.0
            )));
        }

        tx.commit()
            .await
            .map_err(|error| LocalRouteAttachmentError::DetachFailed(error.to_string()))
    }

    async fn detach(
        &self,
        attachment_id: LocalRouteAttachmentId,
    ) -> Result<(), LocalRouteAttachmentError> {
        ChannelProviderModel::delete(self.db.as_ref(), attachment_id.0)
            .await
            .map_err(|error| LocalRouteAttachmentError::DetachFailed(error.to_string()))
    }
}
