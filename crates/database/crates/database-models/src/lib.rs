use burncloud_common::types::Channel;
use burncloud_database::{Database, Result};
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
            "postgres" => r#"
                INSERT INTO prices (model, input_price, output_price, currency, alias_for, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT(model) DO UPDATE SET
                    input_price = EXCLUDED.input_price,
                    output_price = EXCLUDED.output_price,
                    currency = EXCLUDED.currency,
                    alias_for = EXCLUDED.alias_for,
                    updated_at = EXCLUDED.updated_at
            "#,
            _ => r#"
                INSERT INTO prices (model, input_price, output_price, currency, alias_for, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(model) DO UPDATE SET
                    input_price = excluded.input_price,
                    output_price = excluded.output_price,
                    currency = excluded.currency,
                    alias_for = excluded.alias_for,
                    updated_at = excluded.updated_at
            "#,
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

        sqlx::query(sql)
            .bind(model)
            .execute(conn.pool())
            .await?;

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
                INSERT INTO channels ({}, key, status, name, weight, base_url, models, {}, priority, created_time, param_override, header_override)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                RETURNING id
                "#,
                type_col, group_col
            )
        } else {
            format!(
                r#"
                INSERT INTO channels ({}, key, status, name, weight, base_url, models, {}, priority, created_time, param_override, header_override)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
            .bind(&channel.header_override);

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
                SET {} = $1, key = $2, status = $3, name = $4, weight = $5, base_url = $6, models = $7, {} = $8, priority = $9, param_override = $10, header_override = $11
                WHERE id = $12
                "#,
                type_col, group_col
            )
        } else {
            format!(
                r#"
                UPDATE channels 
                SET {} = ?, key = ?, status = ?, name = ?, weight = ?, base_url = ?, models = ?, {} = ?, priority = ?, param_override = ?, header_override = ?
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
                    header_override, remark 
                FROM channels WHERE id = $1
            "#
            }
            _ => {
                r#"
                SELECT 
                    id, type as type_, key, status, name, weight, created_time, test_time, 
                    response_time, base_url, models, `group`, used_quota, model_mapping, 
                    priority, auto_ban, other_info, tag, setting, param_override, 
                    header_override, remark 
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
                    header_override, remark 
                FROM channels ORDER BY id DESC LIMIT $1 OFFSET $2
            "#
            }
            _ => {
                r#"
                SELECT 
                    id, type as type_, key, status, name, weight, created_time, test_time, 
                    response_time, base_url, models, `group`, used_quota, model_mapping, 
                    priority, auto_ban, other_info, tag, setting, param_override, 
                    header_override, remark 
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
