use crate::common::current_timestamp;
use burncloud_database::{adapt_sql, Database, Result};
use serde::{Deserialize, Serialize};

/// Persisted row from the legacy `model_capabilities` table.
///
/// Capability fields are Supply-owned truth. `input_price` and `output_price` remain in this
/// row only because the historical table mixed capability and pricing data. Canonical pricing
/// continues to live in Commerce; these two columns are a compatibility projection until a future
/// data migration physically separates the table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct ModelCapability {
    pub id: i64,
    pub model: String,
    pub context_window: Option<i64>,
    pub max_output_tokens: Option<i64>,
    pub supports_vision: bool,
    pub supports_function_calling: bool,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub synced_at: Option<i64>,
}

/// Input used by the single write boundary for `model_capabilities`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCapabilityInput {
    pub model: String,
    pub context_window: Option<i64>,
    pub max_output_tokens: Option<i64>,
    pub supports_vision: bool,
    pub supports_function_calling: bool,
    /// Legacy USD price projection stored by the historical mixed table.
    pub input_price: Option<f64>,
    /// Legacy USD price projection stored by the historical mixed table.
    pub output_price: Option<f64>,
}

/// Supply-owned persistence boundary for model capability truth.
pub struct ModelCapabilityModel;

impl ModelCapabilityModel {
    /// Read one persisted capability row by model name.
    pub async fn get(db: &Database, model: &str) -> Result<Option<ModelCapability>> {
        let conn = db.get_connection()?;
        let sql = adapt_sql(
            db.kind() == "postgres",
            r#"
            SELECT CAST(id AS BIGINT) AS id, model, context_window, max_output_tokens,
                   supports_vision, supports_function_calling,
                   input_price, output_price, synced_at
            FROM model_capabilities
            WHERE model = ?
            "#,
        );

        let capability = sqlx::query_as::<_, ModelCapability>(&sql)
            .bind(model)
            .fetch_optional(conn.pool())
            .await?;

        Ok(capability)
    }

    /// Insert or replace the capability projection for one model.
    ///
    /// This is the only production write entrance for `model_capabilities`; callers provide
    /// domain values and this adapter owns SQL dialect handling and the sync timestamp.
    pub async fn upsert(db: &Database, input: &ModelCapabilityInput) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = adapt_sql(
            db.kind() == "postgres",
            r#"
            INSERT INTO model_capabilities (
                model,
                context_window,
                max_output_tokens,
                supports_vision,
                supports_function_calling,
                input_price,
                output_price,
                synced_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(model) DO UPDATE SET
                context_window = EXCLUDED.context_window,
                max_output_tokens = EXCLUDED.max_output_tokens,
                supports_vision = EXCLUDED.supports_vision,
                supports_function_calling = EXCLUDED.supports_function_calling,
                input_price = EXCLUDED.input_price,
                output_price = EXCLUDED.output_price,
                synced_at = EXCLUDED.synced_at
            "#,
        );

        sqlx::query(&sql)
            .bind(&input.model)
            .bind(input.context_window)
            .bind(input.max_output_tokens)
            .bind(input.supports_vision)
            .bind(input.supports_function_calling)
            .bind(input.input_price)
            .bind(input.output_price)
            .bind(current_timestamp())
            .execute(conn.pool())
            .await?;

        Ok(())
    }
}
