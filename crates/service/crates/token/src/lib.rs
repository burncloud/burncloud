//! # BurnCloud Service Token
//!
//! Token service layer providing business logic for API token management,
//! including validation, quota tracking, CRUD operations, and key rotation.

use burncloud_database::Database;
use burncloud_database_router::token::{RouterTokenModel, TokenRotationResult};

pub use burncloud_database_router::token::{RouterToken, RouterTokenValidationResult};

type Result<T> = std::result::Result<T, burncloud_database::DatabaseError>;

/// Token service for managing API tokens
pub struct TokenService;

impl TokenService {
    /// List all tokens
    pub async fn list(db: &Database) -> Result<Vec<RouterToken>> {
        RouterTokenModel::list(db).await
    }

    /// Create a new token
    pub async fn create(db: &Database, token: &RouterToken) -> Result<()> {
        RouterTokenModel::create(db, token).await
    }

    /// Delete a token
    pub async fn delete(db: &Database, token: &str) -> Result<()> {
        RouterTokenModel::delete(db, token).await
    }

    /// Update token status
    pub async fn update_status(db: &Database, token: &str, status: &str) -> Result<()> {
        RouterTokenModel::update_status(db, token, status).await
    }

    /// Validate a token and return the token data if valid
    pub async fn validate(db: &Database, token: &str) -> Result<Option<RouterToken>> {
        RouterTokenModel::validate(db, token).await
    }

    /// Validates a token and returns detailed result
    pub async fn validate_detailed(
        db: &Database,
        token: &str,
    ) -> Result<RouterTokenValidationResult> {
        RouterTokenModel::validate_detailed(db, token).await
    }

    /// Update the accessed_time for a token
    pub async fn update_accessed_time(db: &Database, token: &str) -> Result<()> {
        RouterTokenModel::update_accessed_time(db, token).await
    }

    /// Check if quota is sufficient without deducting.
    /// Cost parameter uses i64 nanodollars for precision.
    pub async fn check_quota(db: &Database, token: &str, cost: i64) -> Result<bool> {
        RouterTokenModel::check_quota(db, token, cost).await
    }

    /// Deduct quota from token atomically.
    /// Cost is in quota units (typically 1 quota = 1 token, or can be scaled).
    /// Cost parameter uses i64 nanodollars for precision.
    pub async fn deduct_quota(db: &Database, token: &str, cost: i64) -> Result<bool> {
        RouterTokenModel::deduct_quota(db, token, cost).await
    }

    /// Rotate a token - generates a new key while keeping the old key valid during transition period
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `token` - Current token string
    /// * `transition_period_hours` - Hours the old key remains valid (default 24 if 0)
    /// * `revoke_old` - Whether to immediately revoke the old key
    ///
    /// # Returns
    /// * `Ok(TokenRotationResult)` - Contains new token and rotation info
    /// * `Err(DatabaseError::NotFound)` - Token not found
    pub async fn rotate(
        db: &Database,
        token: &str,
        transition_period_hours: i32,
        revoke_old: bool,
    ) -> Result<TokenRotationResult> {
        RouterTokenModel::rotate(db, token, transition_period_hours, revoke_old).await
    }

    /// Revoke old key version immediately
    pub async fn revoke_old_key(db: &Database, token: &str) -> Result<bool> {
        RouterTokenModel::revoke_old_key(db, token).await
    }

    /// Set IP whitelist for a token
    pub async fn set_ip_whitelist(db: &Database, token: &str, ip_whitelist: &str) -> Result<bool> {
        RouterTokenModel::set_ip_whitelist(db, token, ip_whitelist).await
    }

    /// Check if IP is allowed for token
    pub async fn is_ip_allowed(db: &Database, token: &str, client_ip: &str) -> Result<bool> {
        RouterTokenModel::is_ip_allowed(db, token, client_ip).await
    }
}
