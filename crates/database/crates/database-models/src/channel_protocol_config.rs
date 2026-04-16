use crate::common::current_timestamp;
use burncloud_database::{ph, phs, Database, Result};
use serde::{Deserialize, Serialize};

/// Protocol configuration for dynamic protocol adapters
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChannelProtocolConfig {
    pub id: i32,
    pub channel_type: i32,
    pub api_version: String,
    pub is_default: bool,
    pub chat_endpoint: Option<String>,
    pub embed_endpoint: Option<String>,
    pub models_endpoint: Option<String>,
    pub request_mapping: Option<String>,
    pub response_mapping: Option<String>,
    pub detection_rules: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

/// Input for creating/updating a channel protocol config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelProtocolConfigInput {
    pub channel_type: i32,
    pub api_version: String,
    pub is_default: Option<bool>,
    pub chat_endpoint: Option<String>,
    pub embed_endpoint: Option<String>,
    pub models_endpoint: Option<String>,
    pub request_mapping: Option<String>,
    pub response_mapping: Option<String>,
    pub detection_rules: Option<String>,
}

pub struct ChannelProtocolConfigModel;

impl ChannelProtocolConfigModel {
    /// Get protocol config by channel type and API version
    pub async fn get_by_type_version(
        db: &Database,
        channel_type: i32,
        api_version: &str,
    ) -> Result<Option<ChannelProtocolConfig>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = format!(
            r#"
                SELECT id, channel_type, api_version, is_default, chat_endpoint, embed_endpoint,
                       models_endpoint, request_mapping, response_mapping, detection_rules,
                       created_at, updated_at
                FROM channel_protocol_configs
                WHERE channel_type = {} AND api_version = {}
                "#,
            ph(is_postgres, 1),
            ph(is_postgres, 2)
        );

        let config = sqlx::query_as(&sql)
            .bind(channel_type)
            .bind(api_version)
            .fetch_optional(conn.pool())
            .await?;

        Ok(config)
    }

    /// Get the default protocol config for a channel type
    pub async fn get_default(
        db: &Database,
        channel_type: i32,
    ) -> Result<Option<ChannelProtocolConfig>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = if is_postgres {
            format!(
                r#"
                SELECT id, channel_type, api_version, is_default, chat_endpoint, embed_endpoint,
                       models_endpoint, request_mapping, response_mapping, detection_rules,
                       created_at, updated_at
                FROM channel_protocol_configs
                WHERE channel_type = {} AND is_default = TRUE
                "#,
                ph(is_postgres, 1)
            )
        } else {
            format!(
                r#"
                SELECT id, channel_type, api_version, is_default, chat_endpoint, embed_endpoint,
                       models_endpoint, request_mapping, response_mapping, detection_rules,
                       created_at, updated_at
                FROM channel_protocol_configs
                WHERE channel_type = {} AND is_default = 1
                "#,
                ph(is_postgres, 1)
            )
        };

        let config = sqlx::query_as(&sql)
            .bind(channel_type)
            .fetch_optional(conn.pool())
            .await?;

        Ok(config)
    }

    /// List all protocol configs
    pub async fn list(
        db: &Database,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ChannelProtocolConfig>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = format!(
            r#"
                SELECT id, channel_type, api_version, is_default, chat_endpoint, embed_endpoint,
                       models_endpoint, request_mapping, response_mapping, detection_rules,
                       created_at, updated_at
                FROM channel_protocol_configs
                ORDER BY channel_type, api_version
                LIMIT {} OFFSET {}
                "#,
            ph(is_postgres, 1),
            ph(is_postgres, 2)
        );

        let configs = sqlx::query_as(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(conn.pool())
            .await?;

        Ok(configs)
    }

    /// Create or update a protocol config (upsert)
    pub async fn upsert(db: &Database, input: &ChannelProtocolConfigInput) -> Result<()> {
        let conn = db.get_connection()?;
        let now = current_timestamp();
        let is_postgres = db.kind() == "postgres";

        let is_default = input.is_default.unwrap_or(false);

        // If this is set as default, clear other defaults for the same channel type
        if is_default {
            let clear_sql = format!(
                "UPDATE channel_protocol_configs SET is_default = {} WHERE channel_type = {}",
                if is_postgres { "FALSE" } else { "0" },
                ph(is_postgres, 1)
            );
            sqlx::query(&clear_sql)
                .bind(input.channel_type)
                .execute(conn.pool())
                .await?;
        }

        let sql = format!(
            r#"
                INSERT INTO channel_protocol_configs (channel_type, api_version, is_default, chat_endpoint,
                    embed_endpoint, models_endpoint, request_mapping, response_mapping,
                    detection_rules, created_at, updated_at)
                VALUES ({})
                ON CONFLICT(channel_type, api_version) DO UPDATE SET
                    is_default = EXCLUDED.is_default,
                    chat_endpoint = EXCLUDED.chat_endpoint,
                    embed_endpoint = EXCLUDED.embed_endpoint,
                    models_endpoint = EXCLUDED.models_endpoint,
                    request_mapping = EXCLUDED.request_mapping,
                    response_mapping = EXCLUDED.response_mapping,
                    detection_rules = EXCLUDED.detection_rules,
                    updated_at = EXCLUDED.updated_at
                "#,
            phs(is_postgres, 11)
        );

        sqlx::query(&sql)
            .bind(input.channel_type)
            .bind(&input.api_version)
            .bind(is_default)
            .bind(&input.chat_endpoint)
            .bind(&input.embed_endpoint)
            .bind(&input.models_endpoint)
            .bind(&input.request_mapping)
            .bind(&input.response_mapping)
            .bind(&input.detection_rules)
            .bind(now)
            .bind(now)
            .execute(conn.pool())
            .await?;

        Ok(())
    }

    /// Delete a protocol config
    pub async fn delete(db: &Database, id: i32) -> Result<bool> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = format!(
            "DELETE FROM channel_protocol_configs WHERE id = {}",
            ph(is_postgres, 1)
        );

        let result = sqlx::query(&sql).bind(id).execute(conn.pool()).await?;

        Ok(result.rows_affected() > 0)
    }
}
