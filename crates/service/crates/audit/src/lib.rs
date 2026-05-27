//! Audit logging service for BurnCloud.
//!
//! This module provides high-level audit logging functionality for tracking
//! sensitive operations throughout the application.

use burncloud_database::Database;
use burncloud_database_audit::{AuditDatabase, AuditLogInput, AuditLogQuery};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("Database error: {0}")]
    Database(#[from] burncloud_database::DatabaseError),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

pub type AuditResult<T> = Result<T, AuditError>;

/// Action types for audit events.
pub mod actions {
    // Authentication events
    pub const USER_LOGIN: &str = "user.login";
    pub const USER_LOGOUT: &str = "user.logout";
    pub const USER_LOGIN_FAILED: &str = "user.login_failed";
    
    // Token events
    pub const TOKEN_CREATE: &str = "token.create";
    pub const TOKEN_DELETE: &str = "token.delete";
    pub const TOKEN_ROTATE: &str = "token.rotate";
    
    // Channel events
    pub const CHANNEL_CREATE: &str = "channel.create";
    pub const CHANNEL_UPDATE: &str = "channel.update";
    pub const CHANNEL_DELETE: &str = "channel.delete";
    
    // User management events
    pub const USER_CREATE: &str = "user.create";
    pub const USER_UPDATE: &str = "user.update";
    pub const USER_DELETE: &str = "user.delete";
    pub const USER_ROLE_CHANGE: &str = "user.role_change";
    
    // Billing events
    pub const BILLING_QUOTA_ADJUST: &str = "billing.quota_adjust";
    pub const BILLING_REFUND: &str = "billing.refund";
    
    // Configuration events
    pub const CONFIG_CHANGE: &str = "config.change";
    
    // Data export events
    pub const DATA_EXPORT: &str = "data.export";
    pub const DATA_BATCH_QUERY: &str = "data.batch_query";
    
    // Admin access events
    pub const ADMIN_ACCESS: &str = "admin.access";
}

/// Resource types for audit events.
pub mod resources {
    pub const USER: &str = "user";
    pub const TOKEN: &str = "token";
    pub const CHANNEL: &str = "channel";
    pub const BILLING: &str = "billing";
    pub const CONFIG: &str = "config";
    pub const DATA: &str = "data";
    pub const ADMIN: &str = "admin";
}

/// Status values for audit events.
pub mod status {
    pub const SUCCESS: &str = "success";
    pub const FAILURE: &str = "failure";
}

/// High-level audit service for recording and querying audit events.
pub struct AuditService;

impl AuditService {
    /// Record an audit event.
    pub async fn log(
        db: &Database,
        actor_id: impl Into<String>,
        actor_ip: impl Into<String>,
        action: impl Into<String>,
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
        status: impl Into<String>,
        changes: Option<serde_json::Value>,
        error_message: Option<String>,
        metadata: Option<serde_json::Value>,
    ) -> AuditResult<i64> {
        let input = AuditLogInput {
            actor_id: actor_id.into(),
            actor_ip: actor_ip.into(),
            action: action.into(),
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
            status: status.into(),
            changes,
            error_message,
            metadata,
        };

        let id = AuditDatabase::create(db, input.clone()).await?;
        tracing::debug!(
            audit_id = id,
            actor_id = %input.actor_id,
            action = %input.action,
            "Audit event recorded"
        );
        Ok(id)
    }

    /// Query audit logs with filters.
    pub async fn query(db: &Database, query: AuditLogQuery) -> AuditResult<Vec<burncloud_database_audit::AuditLog>> {
        let logs = AuditDatabase::query(db, query).await?;
        Ok(logs)
    }

    /// Get a single audit log by ID.
    pub async fn get_by_id(db: &Database, id: i64) -> AuditResult<Option<burncloud_database_audit::AuditLog>> {
        let log = AuditDatabase::get_by_id(db, id).await?;
        Ok(log)
    }

    /// Count audit logs matching the query.
    pub async fn count(db: &Database, query: AuditLogQuery) -> AuditResult<i64> {
        let count = AuditDatabase::count(db, query).await?;
        Ok(count)
    }

    /// Log a user login event.
    pub async fn log_login(
        db: &Database,
        user_id: impl Into<String>,
        ip: impl Into<String>,
        success: bool,
        error_message: Option<String>,
    ) -> AuditResult<i64> {
        let user_id = user_id.into();
        Self::log(
            db,
            &user_id,
            ip,
            if success { actions::USER_LOGIN } else { actions::USER_LOGIN_FAILED },
            resources::USER,
            &user_id,
            if success { status::SUCCESS } else { status::FAILURE },
            None,
            error_message,
            None,
        ).await
    }

    /// Log a user logout event.
    pub async fn log_logout(
        db: &Database,
        user_id: impl Into<String>,
        ip: impl Into<String>,
    ) -> AuditResult<i64> {
        let user_id = user_id.into();
        Self::log(
            db,
            &user_id,
            ip,
            actions::USER_LOGOUT,
            resources::USER,
            &user_id,
            status::SUCCESS,
            None,
            None,
            None,
        ).await
    }

    /// Log a token creation event.
    pub async fn log_token_create(
        db: &Database,
        user_id: impl Into<String>,
        ip: impl Into<String>,
        token_id: i64,
        token_name: Option<&str>,
    ) -> AuditResult<i64> {
        Self::log(
            db,
            user_id,
            ip,
            actions::TOKEN_CREATE,
            resources::TOKEN,
            token_id.to_string(),
            status::SUCCESS,
            None,
            None,
            Some(serde_json::json!({ "token_name": token_name })),
        ).await
    }

    /// Log a token deletion event.
    pub async fn log_token_delete(
        db: &Database,
        user_id: impl Into<String>,
        ip: impl Into<String>,
        token_id: i64,
    ) -> AuditResult<i64> {
        Self::log(
            db,
            user_id,
            ip,
            actions::TOKEN_DELETE,
            resources::TOKEN,
            token_id.to_string(),
            status::SUCCESS,
            None,
            None,
            None,
        ).await
    }

    /// Log a channel creation event.
    pub async fn log_channel_create(
        db: &Database,
        user_id: impl Into<String>,
        ip: impl Into<String>,
        channel_id: i64,
        channel_name: &str,
    ) -> AuditResult<i64> {
        Self::log(
            db,
            user_id,
            ip,
            actions::CHANNEL_CREATE,
            resources::CHANNEL,
            channel_id.to_string(),
            status::SUCCESS,
            None,
            None,
            Some(serde_json::json!({ "channel_name": channel_name })),
        ).await
    }

    /// Log a channel update event.
    pub async fn log_channel_update(
        db: &Database,
        user_id: impl Into<String>,
        ip: impl Into<String>,
        channel_id: i64,
        changes: serde_json::Value,
    ) -> AuditResult<i64> {
        Self::log(
            db,
            user_id,
            ip,
            actions::CHANNEL_UPDATE,
            resources::CHANNEL,
            channel_id.to_string(),
            status::SUCCESS,
            Some(changes),
            None,
            None,
        ).await
    }

    /// Log a channel deletion event.
    pub async fn log_channel_delete(
        db: &Database,
        user_id: impl Into<String>,
        ip: impl Into<String>,
        channel_id: i64,
        channel_name: &str,
    ) -> AuditResult<i64> {
        Self::log(
            db,
            user_id,
            ip,
            actions::CHANNEL_DELETE,
            resources::CHANNEL,
            channel_id.to_string(),
            status::SUCCESS,
            None,
            None,
            Some(serde_json::json!({ "channel_name": channel_name })),
        ).await
    }

    /// Log a user creation event.
    pub async fn log_user_create(
        db: &Database,
        actor_id: impl Into<String>,
        ip: impl Into<String>,
        new_user_id: impl Into<String>,
    ) -> AuditResult<i64> {
        Self::log(
            db,
            actor_id,
            ip,
            actions::USER_CREATE,
            resources::USER,
            new_user_id,
            status::SUCCESS,
            None,
            None,
            None,
        ).await
    }

    /// Log a user update event.
    pub async fn log_user_update(
        db: &Database,
        actor_id: impl Into<String>,
        ip: impl Into<String>,
        user_id: impl Into<String>,
        changes: serde_json::Value,
    ) -> AuditResult<i64> {
        Self::log(
            db,
            actor_id,
            ip,
            actions::USER_UPDATE,
            resources::USER,
            user_id,
            status::SUCCESS,
            Some(changes),
            None,
            None,
        ).await
    }

    /// Log a user deletion event.
    pub async fn log_user_delete(
        db: &Database,
        actor_id: impl Into<String>,
        ip: impl Into<String>,
        deleted_user_id: impl Into<String>,
    ) -> AuditResult<i64> {
        Self::log(
            db,
            actor_id,
            ip,
            actions::USER_DELETE,
            resources::USER,
            deleted_user_id,
            status::SUCCESS,
            None,
            None,
            None,
        ).await
    }

    /// Log a quota adjustment event.
    pub async fn log_quota_adjust(
        db: &Database,
        actor_id: impl Into<String>,
        ip: impl Into<String>,
        user_id: impl Into<String>,
        amount: i64,
        currency: &str,
        reason: Option<&str>,
    ) -> AuditResult<i64> {
        Self::log(
            db,
            actor_id,
            ip,
            actions::BILLING_QUOTA_ADJUST,
            resources::BILLING,
            user_id,
            status::SUCCESS,
            None,
            None,
            Some(serde_json::json!({
                "amount": amount,
                "currency": currency,
                "reason": reason
            })),
        ).await
    }

    /// Log a data export event.
    pub async fn log_data_export(
        db: &Database,
        user_id: impl Into<String>,
        ip: impl Into<String>,
        export_type: &str,
        record_count: i64,
    ) -> AuditResult<i64> {
        Self::log(
            db,
            user_id,
            ip,
            actions::DATA_EXPORT,
            resources::DATA,
            format!("{}_export", export_type),
            status::SUCCESS,
            None,
            None,
            Some(serde_json::json!({
                "export_type": export_type,
                "record_count": record_count
            })),
        ).await
    }
}

// Re-export types from database crate
pub use burncloud_database_audit::AuditLog;
