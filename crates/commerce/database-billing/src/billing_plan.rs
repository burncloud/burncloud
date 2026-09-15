//! Database model for billing_plans table (Issue #232)
//!
//! Monthly quota billing plans bound to specific upstream channels.

use burncloud_common::types::{BillingMode, BillingPlan, BillingPlanInput};
use burncloud_database::{Database, DatabaseError, Result};
use sqlx::query_as;

pub struct BillingPlanModel;

impl BillingPlanModel {
    /// Create a new billing plan
    pub async fn create(db: &Database, input: &BillingPlanInput) -> Result<BillingPlan> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        // Convert CNY to nanodollars (multiply by 10^9)
        let monthly_fee = input.monthly_fee_cny * 1_000_000_000;

        // Determine quota limit based on billing mode
        let (request_limit, token_limit) = match input.billing_mode {
            BillingMode::PerRequest => (input.request_limit, None),
            BillingMode::PerToken => (None, input.token_limit),
        };

        let billing_mode_str = input.billing_mode.to_string();

        let sql = if is_postgres {
            r#"INSERT INTO billing_plans (name, monthly_fee, billing_mode, request_limit, token_limit, channel_id)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id, name, monthly_fee, billing_mode, request_limit, token_limit, channel_id, created_at, updated_at"#
        } else {
            r#"INSERT INTO billing_plans (name, monthly_fee, billing_mode, request_limit, token_limit, channel_id)
               VALUES (?, ?, ?, ?, ?, ?)
               RETURNING id, name, monthly_fee, billing_mode, request_limit, token_limit, channel_id, created_at, updated_at"#
        };

        query_as::<_, BillingPlan>(sql)
            .bind(&input.name)
            .bind(monthly_fee)
            .bind(&billing_mode_str)
            .bind(request_limit)
            .bind(token_limit)
            .bind(input.channel_id)
            .fetch_one(conn.pool())
            .await
            .map_err(|e| DatabaseError::Query(format!("Failed to create billing plan: {}", e)))
    }

    /// Get a billing plan by ID
    pub async fn get_by_id(db: &Database, id: i32) -> Result<Option<BillingPlan>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = if is_postgres {
            "SELECT id, name, monthly_fee, billing_mode, request_limit, token_limit, channel_id, created_at, updated_at FROM billing_plans WHERE id = $1"
        } else {
            "SELECT id, name, monthly_fee, billing_mode, request_limit, token_limit, channel_id, created_at, updated_at FROM billing_plans WHERE id = ?"
        };

        query_as::<_, BillingPlan>(sql)
            .bind(id)
            .fetch_optional(conn.pool())
            .await
            .map_err(|e| DatabaseError::Query(format!("Failed to get billing plan: {}", e)))
    }

    /// Get a billing plan by name
    pub async fn get_by_name(db: &Database, name: &str) -> Result<Option<BillingPlan>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = if is_postgres {
            "SELECT id, name, monthly_fee, billing_mode, request_limit, token_limit, channel_id, created_at, updated_at FROM billing_plans WHERE name = $1"
        } else {
            "SELECT id, name, monthly_fee, billing_mode, request_limit, token_limit, channel_id, created_at, updated_at FROM billing_plans WHERE name = ?"
        };

        query_as::<_, BillingPlan>(sql)
            .bind(name)
            .fetch_optional(conn.pool())
            .await
            .map_err(|e| DatabaseError::Query(format!("Failed to get billing plan by name: {}", e)))
    }

    /// List all billing plans
    pub async fn list_all(db: &Database) -> Result<Vec<BillingPlan>> {
        let conn = db.get_connection()?;

        let sql = "SELECT id, name, monthly_fee, billing_mode, request_limit, token_limit, channel_id, created_at, updated_at FROM billing_plans ORDER BY id";

        query_as::<_, BillingPlan>(sql)
            .fetch_all(conn.pool())
            .await
            .map_err(|e| DatabaseError::Query(format!("Failed to list billing plans: {}", e)))
    }

    /// List billing plans by channel ID
    pub async fn list_by_channel(db: &Database, channel_id: i32) -> Result<Vec<BillingPlan>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = if is_postgres {
            "SELECT id, name, monthly_fee, billing_mode, request_limit, token_limit, channel_id, created_at, updated_at FROM billing_plans WHERE channel_id = $1 ORDER BY id"
        } else {
            "SELECT id, name, monthly_fee, billing_mode, request_limit, token_limit, channel_id, created_at, updated_at FROM billing_plans WHERE channel_id = ? ORDER BY id"
        };

        query_as::<_, BillingPlan>(sql)
            .bind(channel_id)
            .fetch_all(conn.pool())
            .await
            .map_err(|e| {
                DatabaseError::Query(format!("Failed to list billing plans by channel: {}", e))
            })
    }

    /// Delete a billing plan
    pub async fn delete(db: &Database, id: i32) -> Result<bool> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = if is_postgres {
            "DELETE FROM billing_plans WHERE id = $1"
        } else {
            "DELETE FROM billing_plans WHERE id = ?"
        };

        let result = sqlx::query(sql)
            .bind(id)
            .execute(conn.pool())
            .await
            .map_err(|e| DatabaseError::Query(format!("Failed to delete billing plan: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }
}
