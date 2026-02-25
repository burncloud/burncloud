use crate::common::current_timestamp;
use burncloud_database::{Database, Result};
use serde::{Deserialize, Serialize};

/// Protocol configuration for dynamic protocol adapters
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProtocolConfig {
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

/// Input for creating/updating a protocol config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfigInput {
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

pub struct ProtocolConfigModel;

impl ProtocolConfigModel {
    /// Get protocol config by channel type and API version
    pub async fn get_by_type_version(
        db: &Database,
        channel_type: i32,
        api_version: &str,
    ) -> Result<Option<ProtocolConfig>> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => {
                r#"
                SELECT id, channel_type, api_version, is_default, chat_endpoint, embed_endpoint,
                       models_endpoint, request_mapping, response_mapping, detection_rules,
                       created_at, updated_at
                FROM protocol_configs
                WHERE channel_type = $1 AND api_version = $2
                "#
            }
            _ => {
                r#"
                SELECT id, channel_type, api_version, is_default, chat_endpoint, embed_endpoint,
                       models_endpoint, request_mapping, response_mapping, detection_rules,
                       created_at, updated_at
                FROM protocol_configs
                WHERE channel_type = ? AND api_version = ?
                "#
            }
        };

        let config = sqlx::query_as(sql)
            .bind(channel_type)
            .bind(api_version)
            .fetch_optional(conn.pool())
            .await?;

        Ok(config)
    }

    /// Get the default protocol config for a channel type
    pub async fn get_default(db: &Database, channel_type: i32) -> Result<Option<ProtocolConfig>> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => {
                r#"
                SELECT id, channel_type, api_version, is_default, chat_endpoint, embed_endpoint,
                       models_endpoint, request_mapping, response_mapping, detection_rules,
                       created_at, updated_at
                FROM protocol_configs
                WHERE channel_type = $1 AND is_default = TRUE
                "#
            }
            _ => {
                r#"
                SELECT id, channel_type, api_version, is_default, chat_endpoint, embed_endpoint,
                       models_endpoint, request_mapping, response_mapping, detection_rules,
                       created_at, updated_at
                FROM protocol_configs
                WHERE channel_type = ? AND is_default = 1
                "#
            }
        };

        let config = sqlx::query_as(sql)
            .bind(channel_type)
            .fetch_optional(conn.pool())
            .await?;

        Ok(config)
    }

    /// List all protocol configs
    pub async fn list(db: &Database, limit: i32, offset: i32) -> Result<Vec<ProtocolConfig>> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => {
                r#"
                SELECT id, channel_type, api_version, is_default, chat_endpoint, embed_endpoint,
                       models_endpoint, request_mapping, response_mapping, detection_rules,
                       created_at, updated_at
                FROM protocol_configs
                ORDER BY channel_type, api_version
                LIMIT $1 OFFSET $2
                "#
            }
            _ => {
                r#"
                SELECT id, channel_type, api_version, is_default, chat_endpoint, embed_endpoint,
                       models_endpoint, request_mapping, response_mapping, detection_rules,
                       created_at, updated_at
                FROM protocol_configs
                ORDER BY channel_type, api_version
                LIMIT ? OFFSET ?
                "#
            }
        };

        let configs = sqlx::query_as(sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(conn.pool())
            .await?;

        Ok(configs)
    }

    /// Create or update a protocol config (upsert)
    pub async fn upsert(db: &Database, input: &ProtocolConfigInput) -> Result<()> {
        let conn = db.get_connection()?;
        let now = current_timestamp();

        let is_default = input.is_default.unwrap_or(false);

        // If this is set as default, clear other defaults for the same channel type
        if is_default {
            let clear_sql = match db.kind().as_str() {
                "postgres" => {
                    "UPDATE protocol_configs SET is_default = FALSE WHERE channel_type = $1"
                }
                _ => "UPDATE protocol_configs SET is_default = 0 WHERE channel_type = ?",
            };
            sqlx::query(clear_sql)
                .bind(input.channel_type)
                .execute(conn.pool())
                .await?;
        }

        let sql = match db.kind().as_str() {
            "postgres" => {
                r#"
                INSERT INTO protocol_configs (channel_type, api_version, is_default, chat_endpoint,
                    embed_endpoint, models_endpoint, request_mapping, response_mapping,
                    detection_rules, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                ON CONFLICT(channel_type, api_version) DO UPDATE SET
                    is_default = EXCLUDED.is_default,
                    chat_endpoint = EXCLUDED.chat_endpoint,
                    embed_endpoint = EXCLUDED.embed_endpoint,
                    models_endpoint = EXCLUDED.models_endpoint,
                    request_mapping = EXCLUDED.request_mapping,
                    response_mapping = EXCLUDED.response_mapping,
                    detection_rules = EXCLUDED.detection_rules,
                    updated_at = EXCLUDED.updated_at
                "#
            }
            _ => {
                r#"
                INSERT INTO protocol_configs (channel_type, api_version, is_default, chat_endpoint,
                    embed_endpoint, models_endpoint, request_mapping, response_mapping,
                    detection_rules, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(channel_type, api_version) DO UPDATE SET
                    is_default = excluded.is_default,
                    chat_endpoint = excluded.chat_endpoint,
                    embed_endpoint = excluded.embed_endpoint,
                    models_endpoint = excluded.models_endpoint,
                    request_mapping = excluded.request_mapping,
                    response_mapping = excluded.response_mapping,
                    detection_rules = excluded.detection_rules,
                    updated_at = excluded.updated_at
                "#
            }
        };

        sqlx::query(sql)
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
        let sql = match db.kind().as_str() {
            "postgres" => "DELETE FROM protocol_configs WHERE id = $1",
            _ => "DELETE FROM protocol_configs WHERE id = ?",
        };

        let result = sqlx::query(sql).bind(id).execute(conn.pool()).await?;

        Ok(result.rows_affected() > 0)
    }
}
