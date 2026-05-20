//! Subscription service for monthly quota billing (Issue #232)
//!
//! Provides business logic for managing billing plans and subscriptions.

use burncloud_common::types::{
    BillingPlan, BillingPlanInput, BillingSubscription, SubscriptionStatus,
    SubscriptionStatusResponse,
};
use burncloud_database::{Database, DatabaseError};
use burncloud_database_billing::{BillingPlanModel, BillingSubscriptionModel};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SubscriptionError {
    #[error("Plan not found: {0}")]
    PlanNotFound(i32),
    #[error("User not found: {0}")]
    UserNotFound(i32),
    #[error("User already has active subscription for channel {0}")]
    AlreadySubscribed(i32),
    #[error("Quota exceeded")]
    QuotaExceeded,
    #[error("Subscription expired")]
    SubscriptionExpired,
    #[error("Subscription not found: {0}")]
    SubscriptionNotFound(i32),
    #[error("Database error: {0}")]
    DatabaseError(#[from] DatabaseError),
}

pub type SubscriptionResult<T> = Result<T, SubscriptionError>;

/// Subscription service for managing billing plans and user subscriptions
pub struct SubscriptionService;

impl SubscriptionService {
    /// Create a new billing plan
    pub async fn create_plan(
        db: &Database,
        input: BillingPlanInput,
    ) -> SubscriptionResult<BillingPlan> {
        BillingPlanModel::create(db, &input)
            .await
            .map_err(SubscriptionError::from)
    }

    /// Get a billing plan by ID
    pub async fn get_plan(db: &Database, plan_id: i32) -> SubscriptionResult<BillingPlan> {
        BillingPlanModel::get_by_id(db, plan_id)
            .await?
            .ok_or(SubscriptionError::PlanNotFound(plan_id))
    }

    /// List all billing plans
    pub async fn list_plans(db: &Database) -> SubscriptionResult<Vec<BillingPlan>> {
        BillingPlanModel::list_all(db)
            .await
            .map_err(SubscriptionError::from)
    }

    /// List billing plans for a specific channel
    pub async fn list_plans_by_channel(
        db: &Database,
        channel_id: i32,
    ) -> SubscriptionResult<Vec<BillingPlan>> {
        BillingPlanModel::list_by_channel(db, channel_id)
            .await
            .map_err(SubscriptionError::from)
    }

    /// Delete a billing plan
    pub async fn delete_plan(db: &Database, plan_id: i32) -> SubscriptionResult<bool> {
        BillingPlanModel::delete(db, plan_id)
            .await
            .map_err(SubscriptionError::from)
    }

    /// Subscribe a user to a plan
    /// Creates a subscription that inherits the channel_id from the plan
    pub async fn subscribe(
        db: &Database,
        user_id: i32,
        plan_id: i32,
        duration_days: i64,
    ) -> SubscriptionResult<BillingSubscription> {
        // Get the plan
        let plan = BillingPlanModel::get_by_id(db, plan_id)
            .await?
            .ok_or(SubscriptionError::PlanNotFound(plan_id))?;

        // Check if user already has an active subscription for this channel
        if BillingSubscriptionModel::get_active_by_user_channel(db, user_id, plan.channel_id)
            .await?
            .is_some()
        {
            return Err(SubscriptionError::AlreadySubscribed(plan.channel_id));
        }

        // Calculate quota limit based on billing mode
        let quota_limit = match plan.billing_mode.as_str() {
            "per_request" => plan.request_limit.unwrap_or(0),
            "per_token" => plan.token_limit.unwrap_or(0),
            _ => 0,
        };

        // Calculate expiration time
        let now = chrono::Utc::now().timestamp_millis();
        let expires_at = now + (duration_days * 24 * 60 * 60 * 1000);

        // Create the subscription
        BillingSubscriptionModel::create(
            db,
            user_id,
            plan_id,
            quota_limit,
            plan.channel_id,
            expires_at,
        )
        .await
        .map_err(SubscriptionError::from)
    }

    /// Get active subscription for a user
    pub async fn get_active_subscription(
        db: &Database,
        user_id: i32,
    ) -> SubscriptionResult<Option<BillingSubscription>> {
        BillingSubscriptionModel::get_active_by_user(db, user_id)
            .await
            .map_err(SubscriptionError::from)
    }

    /// Get active subscription for a user by channel
    /// Used to check if a user can use a specific channel
    pub async fn get_subscription_for_channel(
        db: &Database,
        user_id: i32,
        channel_id: i32,
    ) -> SubscriptionResult<Option<BillingSubscription>> {
        BillingSubscriptionModel::get_active_by_user_channel(db, user_id, channel_id)
            .await
            .map_err(SubscriptionError::from)
    }

    /// Check if a user has quota available for a channel
    /// Returns the subscription if available, None if no subscription or quota exceeded
    pub async fn check_quota(
        db: &Database,
        user_id: i32,
        channel_id: i32,
        required: i64,
    ) -> SubscriptionResult<Option<BillingSubscription>> {
        let sub =
            BillingSubscriptionModel::get_active_by_user_channel(db, user_id, channel_id).await?;

        let sub = match sub {
            Some(s) => s,
            None => return Ok(None),
        };

        // Check if expired
        let now = chrono::Utc::now().timestamp_millis();
        if sub.expires_at <= now {
            return Err(SubscriptionError::SubscriptionExpired);
        }

        // Check quota
        if sub.quota_used + required > sub.quota_limit {
            return Err(SubscriptionError::QuotaExceeded);
        }

        Ok(Some(sub))
    }

    /// Consume quota for a request
    /// This should be called after a successful request
    pub async fn consume_quota(
        db: &Database,
        subscription_id: i32,
        amount: i64,
    ) -> SubscriptionResult<BillingSubscription> {
        BillingSubscriptionModel::increment_quota(db, subscription_id, amount)
            .await
            .map_err(|e| match e {
                DatabaseError::InvalidData { message } if message == "Quota exceeded" => {
                    SubscriptionError::QuotaExceeded
                }
                other => SubscriptionError::DatabaseError(other),
            })
    }

    /// Get subscription status with plan details
    pub async fn get_subscription_status(
        db: &Database,
        subscription_id: i32,
    ) -> SubscriptionResult<SubscriptionStatusResponse> {
        BillingSubscriptionModel::get_status(db, subscription_id)
            .await?
            .ok_or(SubscriptionError::SubscriptionNotFound(subscription_id))
    }

    /// Cancel a subscription
    pub async fn cancel_subscription(
        db: &Database,
        subscription_id: i32,
    ) -> SubscriptionResult<BillingSubscription> {
        BillingSubscriptionModel::cancel(db, subscription_id)
            .await
            .map_err(SubscriptionError::from)
    }

    /// List all subscriptions for a user
    pub async fn list_user_subscriptions(
        db: &Database,
        user_id: i32,
    ) -> SubscriptionResult<Vec<BillingSubscription>> {
        BillingSubscriptionModel::list_by_user(db, user_id)
            .await
            .map_err(SubscriptionError::from)
    }

    /// Expire overdue subscriptions (should be called periodically)
    pub async fn expire_overdue_subscriptions(db: &Database) -> SubscriptionResult<u64> {
        BillingSubscriptionModel::expire_overdue(db)
            .await
            .map_err(SubscriptionError::from)
    }
}
