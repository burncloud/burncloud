use crate::common::current_timestamp;
use burncloud_database::{ph, phs, Database, Result};
use serde::{Deserialize, Serialize};

/// Protocol configuration for dynamic protocol adapters
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChannelProtocolConfig {
    pub id: i32,
    pub channel_type: i32,
    pub api_version: String,
    pub is_default: i32,
    pub chat_endpoint: Option<String>,
    pub embed_endpoint: Option<String>,
    pub models_endpoint: Option<String>,
    pub request_mapping: Option<String>,
    pub response_mapping: Option<String>,
    pub detection_rules: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

impl ChannelProtocolConfig {
    pub fn is_default_bool(&self) -> bool {
        self.is_default != 0
    }
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
        let is_default_column = if is_postgres {
            "is_default::INTEGER AS is_default"
        } else {
            "is_default"
        };
        let sql = format!(
            r#"
                SELECT id, channel_type, api_version, {is_default_column}, chat_endpoint, embed_endpoint,
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
        let is_default_column = if is_postgres {
            "is_default::INTEGER AS is_default"
        } else {
            "is_default"
        };
        let sql = if is_postgres {
            format!(
                r#"
                SELECT id, channel_type, api_version, {is_default_column}, chat_endpoint, embed_endpoint,
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
                SELECT id, channel_type, api_version, {is_default_column}, chat_endpoint, embed_endpoint,
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
        let is_default_column = if is_postgres {
            "is_default::INTEGER AS is_default"
        } else {
            "is_default"
        };
        let sql = format!(
            r#"
                SELECT id, channel_type, api_version, {is_default_column}, chat_endpoint, embed_endpoint,
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

    /// Seed the four default protocol configs that Supply owns (#842).
    ///
    /// `channel_protocol_configs` is Supply's Data Truth, so the default records written into
    /// it belong here rather than in `platform/storage/database`. Platform previously issued
    /// these INSERTs on every database initialization; the SQL is moved verbatim, including
    /// the SQLite/PostgreSQL placeholder split, so the seeded rows are unchanged.
    ///
    /// Idempotent: it only runs while the table is empty, so a second call inserts nothing and
    /// never overwrites a row an operator has since edited. A missing table is treated as
    /// "nothing to seed yet" rather than an error, preserving the previous tolerance during
    /// early bootstrap.
    ///
    /// Ordering is the caller's responsibility: the application bootstrap runs this after
    /// infrastructure initialization, because that is what creates the table.
    pub async fn seed_default_protocol_configs(db: &Database) -> Result<()> {
        let conn = db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = db.kind() == "postgres";

        let pc_count: i64 = match sqlx::query_scalar("SELECT count(*) FROM channel_protocol_configs")
            .fetch_one(pool)
            .await
        {
            Ok(n) => n,
            Err(_) => return Ok(()),
        };

        if pc_count != 0 {
            return Ok(());
        }

        let now = current_timestamp();
        type ProtocolConfig<'a> = (
            i32,
            &'a str,
            bool,
            Option<&'a str>,
            Option<&'a str>,
            Option<&'a str>,
        );

        let default_protocols: [ProtocolConfig; 4] = [
            (
                1,
                "default",
                true,
                Some("/v1/chat/completions"),
                Some("/v1/embeddings"),
                Some("/v1/models"),
            ),
            (2, "2023-06-01", true, Some("/v1/messages"), None, None),
            (
                3,
                "2024-02-01",
                true,
                Some("/deployments/{deployment_id}/chat/completions"),
                Some("/deployments/{deployment_id}/embeddings"),
                Some("/deployments?api-version=2024-02-01"),
            ),
            (
                4,
                "v1",
                true,
                Some("/v1/models/{model}:generateContent"),
                Some("/v1/models/{model}:embedContent"),
                Some("/v1/models"),
            ),
        ];

        let insert_sql = if is_postgres {
            "INSERT INTO channel_protocol_configs \
             (channel_type, api_version, is_default, chat_endpoint, \
              embed_endpoint, models_endpoint, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        } else {
            "INSERT INTO channel_protocol_configs \
             (channel_type, api_version, is_default, chat_endpoint, \
              embed_endpoint, models_endpoint, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        };

        for (channel_type, api_version, is_default, chat_endpoint, embed_endpoint, models_endpoint) in
            default_protocols
        {
            sqlx::query(insert_sql)
                .bind(channel_type)
                .bind(api_version)
                .bind(is_default)
                .bind(chat_endpoint)
                .bind(embed_endpoint)
                .bind(models_endpoint)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
        }

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
