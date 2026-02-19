use burncloud_common::types::{Ability, Channel};
use burncloud_database::{Database, Result};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::Row;

pub use burncloud_database::DatabaseError;

/// Model pricing information
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Price {
    pub id: i32,
    pub model: String,
    pub input_price: f64,
    pub output_price: f64,
    #[serde(default)]
    pub currency: String,
    pub alias_for: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

/// Input for creating/updating a price
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceInput {
    pub model: String,
    pub input_price: f64,
    pub output_price: f64,
    pub currency: Option<String>,
    pub alias_for: Option<String>,
}

pub struct PriceModel;

impl PriceModel {
    /// Get price for a model, resolving aliases
    pub async fn get(db: &Database, model: &str) -> Result<Option<Price>> {
        Self::get_inner(db, model, 0).await
    }

    /// Internal get with recursion depth limit to prevent infinite loops
    async fn get_inner(db: &Database, model: &str, depth: u32) -> Result<Option<Price>> {
        // Prevent infinite recursion from circular aliases
        if depth > 10 {
            return Ok(None);
        }

        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => "SELECT id, model, input_price, output_price, currency, alias_for, created_at, updated_at FROM prices WHERE model = $1",
            _ => "SELECT id, model, input_price, output_price, currency, alias_for, created_at, updated_at FROM prices WHERE model = ?",
        };

        let price: Option<Price> = sqlx::query_as(sql)
            .bind(model)
            .fetch_optional(conn.pool())
            .await?;

        // If this price is an alias, resolve to the target model
        if let Some(ref p) = price {
            if let Some(ref alias_for) = p.alias_for {
                return Box::pin(Self::get_inner(db, alias_for, depth + 1)).await;
            }
        }

        Ok(price)
    }

    /// List all prices
    pub async fn list(db: &Database, limit: i32, offset: i32) -> Result<Vec<Price>> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => "SELECT id, model, input_price, output_price, currency, alias_for, created_at, updated_at FROM prices ORDER BY model LIMIT $1 OFFSET $2",
            _ => "SELECT id, model, input_price, output_price, currency, alias_for, created_at, updated_at FROM prices ORDER BY model LIMIT ? OFFSET ?",
        };

        let prices = sqlx::query_as(sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(conn.pool())
            .await?;

        Ok(prices)
    }

    /// Create or update a price (upsert)
    pub async fn upsert(db: &Database, input: &PriceInput) -> Result<()> {
        let conn = db.get_connection()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let sql = match db.kind().as_str() {
            "postgres" => {
                r#"
                INSERT INTO prices (model, input_price, output_price, currency, alias_for, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT(model) DO UPDATE SET
                    input_price = EXCLUDED.input_price,
                    output_price = EXCLUDED.output_price,
                    currency = EXCLUDED.currency,
                    alias_for = EXCLUDED.alias_for,
                    updated_at = EXCLUDED.updated_at
            "#
            }
            _ => {
                r#"
                INSERT INTO prices (model, input_price, output_price, currency, alias_for, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(model) DO UPDATE SET
                    input_price = excluded.input_price,
                    output_price = excluded.output_price,
                    currency = excluded.currency,
                    alias_for = excluded.alias_for,
                    updated_at = excluded.updated_at
            "#
            }
        };

        sqlx::query(sql)
            .bind(&input.model)
            .bind(input.input_price)
            .bind(input.output_price)
            .bind(input.currency.as_deref().unwrap_or("USD"))
            .bind(&input.alias_for)
            .bind(now)
            .bind(now)
            .execute(conn.pool())
            .await?;

        Ok(())
    }

    /// Delete a price
    pub async fn delete(db: &Database, model: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => "DELETE FROM prices WHERE model = $1",
            _ => "DELETE FROM prices WHERE model = ?",
        };

        sqlx::query(sql).bind(model).execute(conn.pool()).await?;

        Ok(())
    }

    /// Calculate cost for a request
    /// Returns cost in the default currency (USD)
    pub fn calculate_cost(price: &Price, prompt_tokens: u32, completion_tokens: u32) -> f64 {
        // Prices are per 1M tokens
        let input_cost = (prompt_tokens as f64 / 1_000_000.0) * price.input_price;
        let output_cost = (completion_tokens as f64 / 1_000_000.0) * price.output_price;
        input_cost + output_cost
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ModelInfo {
    pub model_id: String,
    pub private: bool,
    pub pipeline_tag: Option<String>,
    pub library_name: Option<String>,
    pub model_type: Option<String>,
    pub downloads: i64,
    pub likes: i64,
    pub sha: Option<String>,
    pub last_modified: Option<String>,
    pub gated: bool,
    pub disabled: bool,
    pub tags: String,
    pub config: String,
    pub widget_data: String,
    pub card_data: String,
    pub transformers_info: String,
    pub siblings: String,
    pub spaces: String,
    pub safetensors: String,
    pub used_storage: i64,
    pub filename: Option<String>,
    pub size: i64,
    pub created_at: String,
    pub updated_at: String,
}

pub struct ModelDatabase {
    db: Database,
}

impl ModelDatabase {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            db: Database::new().await?,
        })
    }

    pub async fn close(self) -> Result<()> {
        self.db.close().await
    }

    pub async fn add_model(&self, _model: &ModelInfo) -> Result<()> {
        Ok(())
    }
    pub async fn update(&self, _model: &ModelInfo) -> Result<()> {
        Ok(())
    }
    pub async fn get_model(&self, _model_id: &str) -> Result<Option<ModelInfo>> {
        Ok(None)
    }
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![])
    }
    pub async fn search_by_pipeline(&self, _pipeline_tag: &str) -> Result<Vec<ModelInfo>> {
        Ok(vec![])
    }
    pub async fn get_popular_models(&self, _limit: i64) -> Result<Vec<ModelInfo>> {
        Ok(vec![])
    }
    pub async fn delete(&self, _model_id: &str) -> Result<()> {
        Ok(())
    }
}

/// Token for API authentication
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Token {
    pub id: i32,
    pub user_id: String,
    pub key: String,
    pub status: i32,
    pub name: Option<String>,
    pub remain_quota: i64,
    pub unlimited_quota: bool,
    pub used_quota: i64,
    pub created_time: Option<i64>,
    pub accessed_time: Option<i64>,
    pub expired_time: i64,
}

/// Input for creating a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInput {
    pub user_id: String,
    pub name: Option<String>,
    pub remain_quota: Option<i64>,
    pub unlimited_quota: Option<bool>,
    pub expired_time: Option<i64>,
}

/// Input for updating a token
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUpdateInput {
    pub name: Option<String>,
    pub remain_quota: Option<i64>,
    pub status: Option<i32>,
    pub expired_time: Option<i64>,
}

pub struct TokenModel;

impl TokenModel {
    /// Generate a unique token key with sk- prefix and 48 random characters
    fn generate_key() -> String {
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut rng = rand::thread_rng();
        let key: String = (0..48)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        format!("sk-{}", key)
    }

    /// Create a new token
    pub async fn create(db: &Database, input: &TokenInput) -> Result<Token> {
        let conn = db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = db.kind() == "postgres";

        let key = Self::generate_key();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let remain_quota = input.remain_quota.unwrap_or(0);
        let unlimited_quota = input.unlimited_quota.unwrap_or(false);
        let expired_time = input.expired_time.unwrap_or(-1);

        let sql = if is_postgres {
            r#"
            INSERT INTO tokens (user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time)
            VALUES ($1, $2, 1, $3, $4, $5, 0, $6, $6, $7)
            RETURNING id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time
            "#
        } else {
            r#"
            INSERT INTO tokens (user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time)
            VALUES (?, ?, 1, ?, ?, ?, 0, ?, ?, ?)
            "#
        };

        let mut tx = pool.begin().await?;

        let token = if is_postgres {
            let row = sqlx::query_as::<_, Token>(sql)
                .bind(&input.user_id)
                .bind(&key)
                .bind(&input.name)
                .bind(remain_quota)
                .bind(unlimited_quota)
                .bind(now)
                .bind(expired_time)
                .fetch_one(&mut *tx)
                .await?;
            tx.commit().await?;
            row
        } else {
            sqlx::query(sql)
                .bind(&input.user_id)
                .bind(&key)
                .bind(&input.name)
                .bind(remain_quota)
                .bind(unlimited_quota)
                .bind(now)
                .bind(now)
                .bind(expired_time)
                .execute(&mut *tx)
                .await?;
            let id: (i64,) = sqlx::query_as("SELECT last_insert_rowid()")
                .fetch_one(&mut *tx)
                .await?;
            tx.commit().await?;
            Token {
                id: id.0 as i32,
                user_id: input.user_id.clone(),
                key,
                status: 1,
                name: input.name.clone(),
                remain_quota,
                unlimited_quota,
                used_quota: 0,
                created_time: Some(now),
                accessed_time: Some(now),
                expired_time,
            }
        };

        Ok(token)
    }

    /// Get a token by key
    pub async fn get_by_key(db: &Database, key: &str) -> Result<Option<Token>> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => "SELECT id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time FROM tokens WHERE key = $1",
            _ => "SELECT id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time FROM tokens WHERE key = ?",
        };

        let token = sqlx::query_as(sql)
            .bind(key)
            .fetch_optional(conn.pool())
            .await?;

        Ok(token)
    }

    /// List tokens with pagination and optional user_id filter
    pub async fn list(
        db: &Database,
        limit: i32,
        offset: i32,
        user_id: Option<&str>,
    ) -> Result<Vec<Token>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let tokens = match user_id {
            Some(uid) => {
                let sql = if is_postgres {
                    "SELECT id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time FROM tokens WHERE user_id = $1 ORDER BY id DESC LIMIT $2 OFFSET $3"
                } else {
                    "SELECT id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time FROM tokens WHERE user_id = ? ORDER BY id DESC LIMIT ? OFFSET ?"
                };
                sqlx::query_as(sql)
                    .bind(uid)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(conn.pool())
                    .await?
            }
            None => {
                let sql = if is_postgres {
                    "SELECT id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time FROM tokens ORDER BY id DESC LIMIT $1 OFFSET $2"
                } else {
                    "SELECT id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time FROM tokens ORDER BY id DESC LIMIT ? OFFSET ?"
                };
                sqlx::query_as(sql)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(conn.pool())
                    .await?
            }
        };

        Ok(tokens)
    }

    /// Update a token by key
    pub async fn update(db: &Database, key: &str, input: &TokenUpdateInput) -> Result<bool> {
        let conn = db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = db.kind() == "postgres";

        // Build dynamic update query
        let mut updates = Vec::new();
        let mut param_count = 1;

        if input.name.is_some() {
            updates.push(if is_postgres {
                format!("name = ${}", param_count)
            } else {
                "name = ?".to_string()
            });
            param_count += 1;
        }
        if input.remain_quota.is_some() {
            updates.push(if is_postgres {
                format!("remain_quota = ${}", param_count)
            } else {
                "remain_quota = ?".to_string()
            });
            param_count += 1;
        }
        if input.status.is_some() {
            updates.push(if is_postgres {
                format!("status = ${}", param_count)
            } else {
                "status = ?".to_string()
            });
            param_count += 1;
        }
        if input.expired_time.is_some() {
            updates.push(if is_postgres {
                format!("expired_time = ${}", param_count)
            } else {
                "expired_time = ?".to_string()
            });
            param_count += 1;
        }

        if updates.is_empty() {
            return Ok(false);
        }

        let key_param = if is_postgres {
            format!("${}", param_count)
        } else {
            "?".to_string()
        };

        let sql = format!(
            "UPDATE tokens SET {} WHERE key = {}",
            updates.join(", "),
            key_param
        );

        let mut query = sqlx::query(&sql);
        if let Some(ref name) = input.name {
            query = query.bind(name);
        }
        if let Some(remain_quota) = input.remain_quota {
            query = query.bind(remain_quota);
        }
        if let Some(status) = input.status {
            query = query.bind(status);
        }
        if let Some(expired_time) = input.expired_time {
            query = query.bind(expired_time);
        }
        query = query.bind(key);

        let result = query.execute(pool).await?;
        Ok(result.rows_affected() > 0)
    }

    /// Delete a token by key
    pub async fn delete(db: &Database, key: &str) -> Result<bool> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => "DELETE FROM tokens WHERE key = $1",
            _ => "DELETE FROM tokens WHERE key = ?",
        };

        let result = sqlx::query(sql).bind(key).execute(conn.pool()).await?;

        Ok(result.rows_affected() > 0)
    }
}

pub struct ChannelModel;

impl ChannelModel {
    pub async fn create(db: &Database, channel: &mut Channel) -> Result<i32> {
        let conn = db.get_connection()?;
        let pool = conn.pool();

        let group_col = if db.kind() == "postgres" {
            "\"group\""
        } else {
            "`group`"
        };
        let type_col = if db.kind() == "postgres" {
            "\"type\""
        } else {
            "type"
        };

        // Basic Insert
        let sql = if db.kind() == "postgres" {
            format!(
                r#"
                INSERT INTO channels ({}, key, status, name, weight, base_url, models, {}, priority, created_time, param_override, header_override, api_version)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                RETURNING id
                "#,
                type_col, group_col
            )
        } else {
            format!(
                r#"
                INSERT INTO channels ({}, key, status, name, weight, base_url, models, {}, priority, created_time, param_override, header_override, api_version)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                type_col, group_col
            )
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        channel.created_time = Some(now);

        // Use transaction to ensure last_insert_rowid works on the same connection
        let mut tx = pool.begin().await?;

        let query = sqlx::query(&sql)
            .bind(channel.type_)
            .bind(&channel.key)
            .bind(channel.status)
            .bind(&channel.name)
            .bind(channel.weight)
            .bind(&channel.base_url)
            .bind(&channel.models)
            .bind(&channel.group)
            .bind(channel.priority)
            .bind(channel.created_time)
            .bind(&channel.param_override)
            .bind(&channel.header_override)
            .bind(&channel.api_version);

        let id = if db.kind() == "postgres" {
            let row = query.fetch_one(&mut *tx).await?;
            row.get::<i32, _>(0)
        } else {
            query.execute(&mut *tx).await?;
            // For SQLite with AnyPool, we need a separate query to get ID on the SAME connection (transaction)
            let row: (i64,) = sqlx::query_as("SELECT last_insert_rowid()")
                .fetch_one(&mut *tx)
                .await?;
            row.0 as i32
        };

        tx.commit().await?;

        channel.id = id;

        Self::sync_abilities(db, channel).await?;

        Ok(id)
    }

    pub async fn update(db: &Database, channel: &Channel) -> Result<()> {
        let conn = db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = db.kind() == "postgres";

        let group_col = if is_postgres { "\"group\"" } else { "`group`" };
        let type_col = if is_postgres { "\"type\"" } else { "type" };

        let sql = if is_postgres {
            format!(
                r#"
                UPDATE channels
                SET {} = $1, key = $2, status = $3, name = $4, weight = $5, base_url = $6, models = $7, {} = $8, priority = $9, param_override = $10, header_override = $11, api_version = $12
                WHERE id = $13
                "#,
                type_col, group_col
            )
        } else {
            format!(
                r#"
                UPDATE channels
                SET {} = ?, key = ?, status = ?, name = ?, weight = ?, base_url = ?, models = ?, {} = ?, priority = ?, param_override = ?, header_override = ?, api_version = ?
                WHERE id = ?
                "#,
                type_col, group_col
            )
        };

        sqlx::query(&sql)
            .bind(channel.type_)
            .bind(&channel.key)
            .bind(channel.status)
            .bind(&channel.name)
            .bind(channel.weight)
            .bind(&channel.base_url)
            .bind(&channel.models)
            .bind(&channel.group)
            .bind(channel.priority)
            .bind(&channel.param_override)
            .bind(&channel.header_override)
            .bind(&channel.api_version)
            .bind(channel.id)
            .execute(pool)
            .await?;

        Self::sync_abilities(db, channel).await?;
        Ok(())
    }

    pub async fn delete(db: &Database, id: i32) -> Result<()> {
        let conn = db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = db.kind() == "postgres";

        // Delete Abilities first
        let sql_abilities = if is_postgres {
            "DELETE FROM abilities WHERE channel_id = $1"
        } else {
            "DELETE FROM abilities WHERE channel_id = ?"
        };
        sqlx::query(sql_abilities).bind(id).execute(pool).await?;

        // Delete Channel
        let sql_channels = if is_postgres {
            "DELETE FROM channels WHERE id = $1"
        } else {
            "DELETE FROM channels WHERE id = ?"
        };
        sqlx::query(sql_channels).bind(id).execute(pool).await?;

        Ok(())
    }

    pub async fn get_by_id(db: &Database, id: i32) -> Result<Option<Channel>> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => {
                r#"
                SELECT
                    id, type as "type_", key, status, name, weight, created_time, test_time,
                    response_time, base_url, models, "group", used_quota, model_mapping,
                    priority, auto_ban, other_info, tag, setting, param_override,
                    header_override, remark, api_version
                FROM channels WHERE id = $1
            "#
            }
            _ => {
                r#"
                SELECT
                    id, type as type_, key, status, name, weight, created_time, test_time,
                    response_time, base_url, models, `group`, used_quota, model_mapping,
                    priority, auto_ban, other_info, tag, setting, param_override,
                    header_override, remark, api_version
                FROM channels WHERE id = ?
            "#
            }
        };

        let channel = sqlx::query_as(sql)
            .bind(id)
            .fetch_optional(conn.pool())
            .await?;

        Ok(channel)
    }

    pub async fn list(db: &Database, limit: i32, offset: i32) -> Result<Vec<Channel>> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => {
                r#"
                SELECT
                    id, type as "type_", key, status, name, weight, created_time, test_time,
                    response_time, base_url, models, "group", used_quota, model_mapping,
                    priority, auto_ban, other_info, tag, setting, param_override,
                    header_override, remark, api_version
                FROM channels ORDER BY id DESC LIMIT $1 OFFSET $2
            "#
            }
            _ => {
                r#"
                SELECT
                    id, type as type_, key, status, name, weight, created_time, test_time,
                    response_time, base_url, models, `group`, used_quota, model_mapping,
                    priority, auto_ban, other_info, tag, setting, param_override,
                    header_override, remark, api_version
                FROM channels ORDER BY id DESC LIMIT ? OFFSET ?
            "#
            }
        };

        let channels = sqlx::query_as(sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(conn.pool())
            .await?;

        Ok(channels)
    }

    pub async fn sync_abilities(db: &Database, channel: &Channel) -> Result<()> {
        let conn = db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = db.kind() == "postgres";

        // 1. Delete existing abilities for this channel
        let sql_delete = if is_postgres {
            "DELETE FROM abilities WHERE channel_id = $1"
        } else {
            "DELETE FROM abilities WHERE channel_id = ?"
        };
        sqlx::query(sql_delete)
            .bind(channel.id)
            .execute(pool)
            .await?;

        // 2. Add new abilities
        if channel.status != 1 {
            // If channel disabled, don't add abilities
            return Ok(());
        }

        let models: Vec<&str> = channel
            .models
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        let groups: Vec<&str> = channel
            .group
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        let group_col = if is_postgres { "\"group\"" } else { "`group`" };

        let sql_insert = if is_postgres {
            format!(
                r#"
                INSERT INTO abilities ({}, model, channel_id, enabled, priority, weight)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
                group_col
            )
        } else {
            format!(
                r#"
                INSERT INTO abilities ({}, model, channel_id, enabled, priority, weight)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
                group_col
            )
        };

        for model in models {
            for group in &groups {
                println!(
                    "ChannelModel: Inserting ability - Model: {}, Group: {}, ChannelID: {}",
                    model, group, channel.id
                );
                sqlx::query(&sql_insert)
                    .bind(group)
                    .bind(model)
                    .bind(channel.id)
                    .bind(true) // sqlx handles boolean mapping
                    .bind(channel.priority)
                    .bind(channel.weight)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }
}

/// Ability model for routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityInput {
    pub group: String,
    pub model: String,
    pub channel_id: i32,
    pub enabled: bool,
    pub priority: i64,
    pub weight: i32,
}

pub struct AbilityModel;

impl AbilityModel {
    /// Create abilities in batch for a channel
    ///
    /// This is more efficient than creating abilities one by one.
    /// Handles conflicts by using INSERT OR IGNORE / ON CONFLICT DO NOTHING.
    pub async fn create_batch(db: &Database, abilities: &[AbilityInput]) -> Result<usize> {
        if abilities.is_empty() {
            return Ok(0);
        }

        let conn = db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = db.kind() == "postgres";
        let group_col = if is_postgres { "\"group\"" } else { "`group`" };

        let mut count = 0;
        for ability in abilities {
            let sql = if is_postgres {
                format!(
                    r#"
                    INSERT INTO abilities ({}, model, channel_id, enabled, priority, weight)
                    VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT ({}, model, channel_id) DO NOTHING
                    "#,
                    group_col, group_col
                )
            } else {
                format!(
                    r#"
                    INSERT OR IGNORE INTO abilities ({}, model, channel_id, enabled, priority, weight)
                    VALUES (?, ?, ?, ?, ?, ?)
                    "#,
                    group_col
                )
            };

            let result = sqlx::query(&sql)
                .bind(&ability.group)
                .bind(&ability.model)
                .bind(ability.channel_id)
                .bind(ability.enabled)
                .bind(ability.priority)
                .bind(ability.weight)
                .execute(pool)
                .await?;

            count += result.rows_affected() as usize;
        }

        Ok(count)
    }

    /// Delete all abilities for a channel
    pub async fn delete_by_channel(db: &Database, channel_id: i32) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => "DELETE FROM abilities WHERE channel_id = $1",
            _ => "DELETE FROM abilities WHERE channel_id = ?",
        };

        sqlx::query(sql)
            .bind(channel_id)
            .execute(conn.pool())
            .await?;

        Ok(())
    }

    /// List abilities for a channel
    pub async fn list_by_channel(db: &Database, channel_id: i32) -> Result<Vec<Ability>> {
        let conn = db.get_connection()?;
        let group_col = if db.kind() == "postgres" {
            "\"group\""
        } else {
            "`group`"
        };

        let sql = match db.kind().as_str() {
            "postgres" => format!(
                "SELECT {} as \"group\", model, channel_id, enabled, priority, weight FROM abilities WHERE channel_id = $1",
                group_col
            ),
            _ => format!(
                "SELECT {} as `group`, model, channel_id, enabled, priority, weight FROM abilities WHERE channel_id = ?",
                group_col
            ),
        };

        let abilities = sqlx::query_as(&sql)
            .bind(channel_id)
            .fetch_all(conn.pool())
            .await?;

        Ok(abilities)
    }
}

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
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

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
