//! # BurnCloud Service Token
//!
//! Token service layer providing business logic for API token management,
//! including validation, quota tracking, and CRUD operations.

use burncloud_database::Database;
use burncloud_database_token::TokenModel;

pub use burncloud_database_token::{DbToken, TokenValidationResult};

type Result<T> = std::result::Result<T, burncloud_database::DatabaseError>;

/// Token service for managing API tokens
pub struct TokenService;

impl TokenService {
    /// List all tokens
    pub async fn list(db: &Database) -> Result<Vec<DbToken>> {
        TokenModel::list(db).await
    }

    /// Create a new token
    pub async fn create(db: &Database, token: &DbToken) -> Result<()> {
        TokenModel::create(db, token).await
    }

    /// Delete a token
    pub async fn delete(db: &Database, token: &str) -> Result<()> {
        TokenModel::delete(db, token).await
    }

    /// Update token status
    pub async fn update_status(db: &Database, token: &str, status: &str) -> Result<()> {
        TokenModel::update_status(db, token, status).await
    }

    /// Validate a token and return the token data if valid
    pub async fn validate(db: &Database, token: &str) -> Result<Option<DbToken>> {
        TokenModel::validate(db, token).await
    }

    /// Validates a token and returns detailed result
    pub async fn validate_detailed(db: &Database, token: &str) -> Result<TokenValidationResult> {
        TokenModel::validate_detailed(db, token).await
    }

    /// Update the accessed_time for a token
    pub async fn update_accessed_time(db: &Database, token: &str) -> Result<()> {
        TokenModel::update_accessed_time(db, token).await
    }

    /// Check if quota is sufficient without deducting.
    /// Cost parameter uses i64 nanodollars for precision.
    pub async fn check_quota(db: &Database, token: &str, cost: i64) -> Result<bool> {
        TokenModel::check_quota(db, token, cost).await
    }

    /// Deduct quota from token atomically.
    /// Cost is in quota units (typically 1 quota = 1 token, or can be scaled).
    /// Cost parameter uses i64 nanodollars for precision.
    pub async fn deduct_quota(db: &Database, token: &str, cost: i64) -> Result<bool> {
        TokenModel::deduct_quota(db, token, cost).await
    }
}
