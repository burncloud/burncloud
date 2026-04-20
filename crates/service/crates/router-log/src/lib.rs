//! # BurnCloud Service Router Log
//!
//! Router log service layer providing business logic for router logs,
//! usage statistics, and balance deductions.

use burncloud_database::Database;
use burncloud_database_router::{BalanceModel, RouterLogModel};

pub use burncloud_database_router::{
    BillingModelSummary, BillingSummary, ModelUsageStats, RouterLog, UsageStats,
};

type Result<T> = std::result::Result<T, burncloud_database::DatabaseError>;

/// Router log service for managing request logs
pub struct RouterLogService;

impl RouterLogService {
    /// Insert a new router log entry
    pub async fn insert(db: &Database, log: &RouterLog) -> Result<()> {
        RouterLogModel::insert(db, log).await
    }

    /// Get logs with pagination
    pub async fn get(db: &Database, limit: i32, offset: i32) -> Result<Vec<RouterLog>> {
        RouterLogModel::get(db, limit, offset).await
    }

    /// Get logs with optional filtering
    pub async fn get_filtered(
        db: &Database,
        user_id: Option<&str>,
        upstream_id: Option<&str>,
        model: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<RouterLog>> {
        RouterLogModel::get_filtered(db, user_id, upstream_id, model, limit, offset).await
    }

    /// Get total usage by user
    pub async fn get_usage_by_user(db: &Database, user_id: &str) -> Result<(i64, i64)> {
        RouterLogModel::get_usage_by_user(db, user_id).await
    }
}

/// Usage statistics service
pub struct UsageStatsService;

impl UsageStatsService {
    /// Get aggregated usage statistics for a user over a time period
    /// Period can be: "day", "week", "month"
    pub async fn get_stats(db: &Database, user_id: &str, period: &str) -> Result<UsageStats> {
        burncloud_database_router::get_usage_stats(db, user_id, period).await
    }

    /// Get usage statistics grouped by model for a user over a time period
    pub async fn get_stats_by_model(
        db: &Database,
        user_id: &str,
        period: &str,
    ) -> Result<Vec<ModelUsageStats>> {
        burncloud_database_router::get_usage_stats_by_model(db, user_id, period).await
    }
}

/// Balance service for dual-currency deduction
pub struct BalanceService;

impl BalanceService {
    /// Deduct from USD balance.
    /// Cost is in nanodollars (i64).
    /// Returns Ok(true) if deduction successful, Ok(false) if insufficient balance.
    pub async fn deduct_usd(db: &Database, user_id: &str, cost_nano: i64) -> Result<bool> {
        BalanceModel::deduct_usd(db, user_id, cost_nano).await
    }

    /// Deduct from CNY balance.
    /// Cost is in nanodollars (i64).
    /// Returns Ok(true) if deduction successful, Ok(false) if insufficient balance.
    pub async fn deduct_cny(db: &Database, user_id: &str, cost_nano: i64) -> Result<bool> {
        BalanceModel::deduct_cny(db, user_id, cost_nano).await
    }

    /// Deduct cost from dual-currency wallet.
    /// Uses the primary currency first (based on cost_currency), then converts from secondary if needed.
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `user_id` - User ID to deduct from
    /// * `cost_nano` - Cost in nanodollars (i64)
    /// * `cost_currency` - Currency of the cost ("USD" or "CNY")
    /// * `exchange_rate_nano` - Exchange rate scaled by 10^9 (e.g., 7.24 CNY/USD = 7_240_000_000)
    ///
    /// # Returns
    /// Ok(true) if deduction successful, Ok(false) if insufficient balance across both currencies.
    pub async fn deduct_dual_currency(
        db: &Database,
        user_id: &str,
        cost_nano: i64,
        cost_currency: &str,
        exchange_rate_nano: i64,
    ) -> Result<bool> {
        BalanceModel::deduct_dual_currency(
            db,
            user_id,
            cost_nano,
            cost_currency,
            exchange_rate_nano,
        )
        .await
    }
}

/// Billing summary service
pub struct BillingService;

impl BillingService {
    /// Get per-model billing summary for a time range.
    /// `start` and `end` are optional ISO-8601 date strings (e.g. "2024-01-01").
    pub async fn get_billing_summary(
        db: &Database,
        start: Option<&str>,
        end: Option<&str>,
    ) -> Result<BillingSummary> {
        burncloud_database_router::get_billing_summary(db, start, end).await
    }
}
