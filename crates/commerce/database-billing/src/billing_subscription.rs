//! Database model for billing_subscriptions table (Issue #232)
//!
//! User subscriptions to monthly quota billing plans.

use burncloud_common::types::{
    BillingSubscription, SubscriptionStatus, SubscriptionStatusResponse,
};
use burncloud_database::{Database, DatabaseError, Result};
use sqlx::query_as;

pub struct BillingSubscriptionModel;

impl BillingSubscriptionModel {
    /// Create a new subscription for a user
    /// The subscription inherits channel_id from the plan
    pub async fn create(
        db: &Database,
        user_id: i32,
        plan_id: i32,
        quota_limit: i64,
        channel_id: i32,
        expires_at: i64,
    ) -> Result<BillingSubscription> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = if is_postgres {
            r#"INSERT INTO billing_subscriptions (user_id, plan_id, channel_id, status, quota_used, quota_limit, expires_at)
               VALUES ($1, $2, $3, 'active', 0, $4, $5)
               RETURNING id, user_id, plan_id, channel_id, status, quota_used, quota_limit, expires_at, created_at, updated_at"#
        } else {
            r#"INSERT INTO billing_subscriptions (user_id, plan_id, channel_id, status, quota_used, quota_limit, expires_at)
               VALUES (?, ?, ?, 'active', 0, ?, ?)
               RETURNING id, user_id, plan_id, channel_id, status, quota_used, quota_limit, expires_at, created_at, updated_at"#
        };

        query_as::<_, BillingSubscription>(sql)
            .bind(user_id)
            .bind(plan_id)
            .bind(channel_id)
            .bind(quota_limit)
            .bind(expires_at)
            .fetch_one(conn.pool())
            .await
            .map_err(|e| DatabaseError::Query(format!("Failed to create subscription: {}", e)))
    }

    /// Get a subscription by ID
    pub async fn get_by_id(db: &Database, id: i32) -> Result<Option<BillingSubscription>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = if is_postgres {
            "SELECT id, user_id, plan_id, channel_id, status, quota_used, quota_limit, expires_at, created_at, updated_at FROM billing_subscriptions WHERE id = $1"
        } else {
            "SELECT id, user_id, plan_id, channel_id, status, quota_used, quota_limit, expires_at, created_at, updated_at FROM billing_subscriptions WHERE id = ?"
        };

        query_as::<_, BillingSubscription>(sql)
            .bind(id)
            .fetch_optional(conn.pool())
            .await
            .map_err(|e| DatabaseError::Query(format!("Failed to get subscription: {}", e)))
    }

    /// Get active subscription for a user
    pub async fn get_active_by_user(
        db: &Database,
        user_id: i32,
    ) -> Result<Option<BillingSubscription>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let now = chrono::Utc::now().timestamp_millis();

        let sql = if is_postgres {
            r#"SELECT id, user_id, plan_id, channel_id, status, quota_used, quota_limit, expires_at, created_at, updated_at
               FROM billing_subscriptions
               WHERE user_id = $1 AND status = 'active' AND expires_at > $2
               ORDER BY expires_at DESC
               LIMIT 1"#
        } else {
            r#"SELECT id, user_id, plan_id, channel_id, status, quota_used, quota_limit, expires_at, created_at, updated_at
               FROM billing_subscriptions
               WHERE user_id = ? AND status = 'active' AND expires_at > ?
               ORDER BY expires_at DESC
               LIMIT 1"#
        };

        query_as::<_, BillingSubscription>(sql)
            .bind(user_id)
            .bind(now)
            .fetch_optional(conn.pool())
            .await
            .map_err(|e| DatabaseError::Query(format!("Failed to get active subscription: {}", e)))
    }

    /// Get active subscription for a user by channel
    /// This is used to check if a user has a valid subscription for a specific channel
    pub async fn get_active_by_user_channel(
        db: &Database,
        user_id: i32,
        channel_id: i32,
    ) -> Result<Option<BillingSubscription>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let now = chrono::Utc::now().timestamp_millis();

        let sql = if is_postgres {
            r#"SELECT id, user_id, plan_id, channel_id, status, quota_used, quota_limit, expires_at, created_at, updated_at
               FROM billing_subscriptions
               WHERE user_id = $1 AND channel_id = $2 AND status = 'active' AND expires_at > $3
               ORDER BY expires_at DESC
               LIMIT 1"#
        } else {
            r#"SELECT id, user_id, plan_id, channel_id, status, quota_used, quota_limit, expires_at, created_at, updated_at
               FROM billing_subscriptions
               WHERE user_id = ? AND channel_id = ? AND status = 'active' AND expires_at > ?
               ORDER BY expires_at DESC
               LIMIT 1"#
        };

        query_as::<_, BillingSubscription>(sql)
            .bind(user_id)
            .bind(channel_id)
            .bind(now)
            .fetch_optional(conn.pool())
            .await
            .map_err(|e| {
                DatabaseError::Query(format!(
                    "Failed to get active subscription by channel: {}",
                    e
                ))
            })
    }

    /// Increment quota used for a subscription
    /// Returns the updated subscription, or error if quota exceeded
    pub async fn increment_quota(
        db: &Database,
        subscription_id: i32,
        amount: i64,
    ) -> Result<BillingSubscription> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        // First check if quota would be exceeded
        let sub = Self::get_by_id(db, subscription_id).await?.ok_or_else(|| {
            DatabaseError::InvalidData {
                message: "Subscription not found".to_string(),
            }
        })?;

        if sub.quota_used + amount > sub.quota_limit {
            return Err(DatabaseError::InvalidData {
                message: "Quota exceeded".to_string(),
            });
        }

        let sql = if is_postgres {
            r#"UPDATE billing_subscriptions
               SET quota_used = quota_used + $1, updated_at = $2
               WHERE id = $3
               RETURNING id, user_id, plan_id, channel_id, status, quota_used, quota_limit, expires_at, created_at, updated_at"#
        } else {
            r#"UPDATE billing_subscriptions
               SET quota_used = quota_used + ?, updated_at = ?
               WHERE id = ?
               RETURNING id, user_id, plan_id, channel_id, status, quota_used, quota_limit, expires_at, created_at, updated_at"#
        };

        let now = chrono::Utc::now().timestamp_millis();

        query_as::<_, BillingSubscription>(sql)
            .bind(amount)
            .bind(now)
            .bind(subscription_id)
            .fetch_one(conn.pool())
            .await
            .map_err(|e| DatabaseError::Query(format!("Failed to increment quota: {}", e)))
    }

    /// Check if subscription has remaining quota
    pub async fn has_quota(db: &Database, subscription_id: i32, required: i64) -> Result<bool> {
        let sub = Self::get_by_id(db, subscription_id).await?.ok_or_else(|| {
            DatabaseError::InvalidData {
                message: "Subscription not found".to_string(),
            }
        })?;

        Ok(sub.quota_used + required <= sub.quota_limit)
    }

    /// Update subscription status
    pub async fn update_status(
        db: &Database,
        subscription_id: i32,
        status: SubscriptionStatus,
    ) -> Result<BillingSubscription> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let status_str = status.to_string();
        let now = chrono::Utc::now().timestamp_millis();

        let sql = if is_postgres {
            r#"UPDATE billing_subscriptions
               SET status = $1, updated_at = $2
               WHERE id = $3
               RETURNING id, user_id, plan_id, channel_id, status, quota_used, quota_limit, expires_at, created_at, updated_at"#
        } else {
            r#"UPDATE billing_subscriptions
               SET status = ?, updated_at = ?
               WHERE id = ?
               RETURNING id, user_id, plan_id, channel_id, status, quota_used, quota_limit, expires_at, created_at, updated_at"#
        };

        query_as::<_, BillingSubscription>(sql)
            .bind(status_str)
            .bind(now)
            .bind(subscription_id)
            .fetch_one(conn.pool())
            .await
            .map_err(|e| {
                DatabaseError::Query(format!("Failed to update subscription status: {}", e))
            })
    }

    /// Cancel a subscription
    pub async fn cancel(db: &Database, subscription_id: i32) -> Result<BillingSubscription> {
        Self::update_status(db, subscription_id, SubscriptionStatus::Cancelled).await
    }

    /// List all subscriptions for a user
    pub async fn list_by_user(db: &Database, user_id: i32) -> Result<Vec<BillingSubscription>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = if is_postgres {
            "SELECT id, user_id, plan_id, channel_id, status, quota_used, quota_limit, expires_at, created_at, updated_at FROM billing_subscriptions WHERE user_id = $1 ORDER BY created_at DESC"
        } else {
            "SELECT id, user_id, plan_id, channel_id, status, quota_used, quota_limit, expires_at, created_at, updated_at FROM billing_subscriptions WHERE user_id = ? ORDER BY created_at DESC"
        };

        query_as::<_, BillingSubscription>(sql)
            .bind(user_id)
            .fetch_all(conn.pool())
            .await
            .map_err(|e| DatabaseError::Query(format!("Failed to list subscriptions: {}", e)))
    }

    /// Get subscription status with plan details
    pub async fn get_status(
        db: &Database,
        subscription_id: i32,
    ) -> Result<Option<SubscriptionStatusResponse>> {
        let sub = match Self::get_by_id(db, subscription_id).await? {
            Some(s) => s,
            None => return Ok(None),
        };

        // Get the plan
        let plan = crate::billing_plan::BillingPlanModel::get_by_id(db, sub.plan_id)
            .await?
            .ok_or_else(|| DatabaseError::InvalidData {
                message: "Plan not found".to_string(),
            })?;

        let now = chrono::Utc::now().timestamp_millis();
        let is_expired = sub.expires_at <= now || sub.status != "active";
        let quota_remaining = sub.quota_limit.saturating_sub(sub.quota_used);
        let days_remaining = ((sub.expires_at - now) / (24 * 60 * 60 * 1000)).max(0);

        Ok(Some(SubscriptionStatusResponse {
            subscription: sub,
            plan,
            quota_remaining,
            days_remaining,
            is_expired,
        }))
    }

    /// Expire subscriptions that have passed their expiration date
    pub async fn expire_overdue(db: &Database) -> Result<u64> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let now = chrono::Utc::now().timestamp_millis();

        let sql = if is_postgres {
            r#"UPDATE billing_subscriptions
               SET status = 'expired', updated_at = $1
               WHERE status = 'active' AND expires_at <= $1"#
        } else {
            r#"UPDATE billing_subscriptions
               SET status = 'expired', updated_at = ?
               WHERE status = 'active' AND expires_at <= ?"#
        };

        let result = sqlx::query(sql)
            .bind(now)
            .execute(conn.pool())
            .await
            .map_err(|e| DatabaseError::Query(format!("Failed to expire subscriptions: {}", e)))?;

        Ok(result.rows_affected())
    }
}
