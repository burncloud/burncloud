//! Database operations for token management
//!
//! This crate handles all database operations related to API tokens,
//! including validation, quota tracking, CRUD operations, and key rotation.

use burncloud_common::CrudRepository;
use burncloud_database::{adapt_sql, phs, Database, DatabaseError, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Token validation result that distinguishes between invalid and expired tokens
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum RouterTokenValidationResult {
    Valid(RouterToken),
    Invalid,
    Expired,
}

/// Result of token rotation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRotationResult {
    /// The newly generated token (shown only once)
    pub new_token: String,
    /// Token prefix (e.g., "bc_live_", "bc_test_")
    pub key_prefix: String,
    /// New key version
    pub key_version: i32,
    /// Transition period end timestamp (0 if no transition)
    pub transition_ends_at: i64,
    /// Old key version (for reference)
    pub old_key_version: i32,
}

/// API Token configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RouterToken {
    pub token: String,
    pub user_id: String,
    pub status: String, // "active", "disabled"
    #[sqlx(default)]
    pub quota_limit: i64, // -1 for unlimited
    #[sqlx(default)]
    pub used_quota: i64,
    #[sqlx(default)]
    pub expired_time: i64, // -1 for never expire, >0 = unix timestamp
    #[sqlx(default)]
    pub accessed_time: i64, // unix timestamp of last access
    #[sqlx(default)]
    pub key_version: i32,
    #[sqlx(default)]
    pub old_key_hash: Option<String>,
    #[sqlx(default)]
    pub old_key_expires_at: i64,
    #[sqlx(default)]
    pub ip_whitelist: Option<String>,
    #[sqlx(default)]
    pub key_prefix: String,
    #[sqlx(default)]
    pub created_at: i64,
    #[sqlx(default)]
    pub last_rotated_at: i64,
}

pub struct RouterTokenModel;

impl RouterTokenModel {
    /// Generate a secure random token with prefix
    fn generate_token(prefix: &str) -> String {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        format!("{}{}", prefix, hex::encode(bytes))
    }

    /// Get current Unix timestamp in seconds
    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    /// List all tokens
    pub async fn list(db: &Database) -> Result<Vec<RouterToken>> {
        let conn = db.get_connection()?;
        let tokens = sqlx::query_as::<_, RouterToken>(
            "SELECT token, user_id, status, quota_limit, used_quota, expired_time, accessed_time, \
             key_version, old_key_hash, old_key_expires_at, ip_whitelist, key_prefix, created_at, last_rotated_at \
             FROM router_tokens",
        )
        .fetch_all(conn.pool())
        .await?;
        Ok(tokens)
    }

    /// Create a new token
    pub async fn create(db: &Database, t: &RouterToken) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = format!(
            "INSERT INTO router_tokens (token, user_id, status, quota_limit, used_quota, expired_time, accessed_time, \
             key_version, old_key_hash, old_key_expires_at, ip_whitelist, key_prefix, created_at, last_rotated_at) \
             VALUES ({})",
            phs(is_postgres, 14)
        );
        sqlx::query(&sql)
            .bind(&t.token)
            .bind(&t.user_id)
            .bind(&t.status)
            .bind(t.quota_limit)
            .bind(t.used_quota)
            .bind(t.expired_time)
            .bind(t.accessed_time)
            .bind(t.key_version)
            .bind(&t.old_key_hash)
            .bind(t.old_key_expires_at)
            .bind(&t.ip_whitelist)
            .bind(&t.key_prefix)
            .bind(t.created_at)
            .bind(t.last_rotated_at)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    /// Delete a token
    pub async fn delete(db: &Database, token: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = adapt_sql(
            db.kind() == "postgres",
            "DELETE FROM router_tokens WHERE token = ?",
        );
        sqlx::query(&sql).bind(token).execute(conn.pool()).await?;
        Ok(())
    }

    /// Update token status
    pub async fn update_status(db: &Database, token: &str, status: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = adapt_sql(
            db.kind() == "postgres",
            "UPDATE router_tokens SET status = ? WHERE token = ?",
        );
        sqlx::query(&sql)
            .bind(status)
            .bind(token)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    /// Find a token by token string
    pub async fn find_by_token(db: &Database, token: &str) -> Result<Option<RouterToken>> {
        let conn = db.get_connection()?;
        let sql = adapt_sql(
            db.kind() == "postgres",
            "SELECT token, user_id, status, quota_limit, used_quota, expired_time, accessed_time, \
             key_version, old_key_hash, old_key_expires_at, ip_whitelist, key_prefix, created_at, last_rotated_at \
             FROM router_tokens WHERE token = ?",
        );
        let result = sqlx::query_as::<_, RouterToken>(&sql)
            .bind(token)
            .fetch_optional(conn.pool())
            .await?;
        Ok(result)
    }

    /// Rotate a token - generates a new key while keeping the old key valid during transition period
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `token` - Current token string
    /// * `transition_period_hours` - Hours the old key remains valid (0 = immediate invalidation)
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
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        // Get current token info
        let current = Self::find_by_token(db, token)
            .await?
            .ok_or_else(|| DatabaseError::Query("Token not found".to_string()))?;

        let now = Self::current_timestamp();
        let prefix = current.key_prefix.clone();
        let old_version = current.key_version;
        let new_version = old_version + 1;

        // Generate new token
        let new_token = Self::generate_token(&prefix);

        // Calculate transition end time
        let transition_ends_at = if revoke_old {
            0 // Immediate invalidation
        } else if transition_period_hours > 0 {
            now + (transition_period_hours as i64 * 3600)
        } else {
            // Default 24 hours
            now + (24 * 3600)
        };

        // Hash the old token for transition period validation
        let old_key_hash = if revoke_old {
            None
        } else {
            Some(format!("{:x}", md5::compute(token.as_bytes())))
        };

        // Update the token record
        let mut tx = conn.pool().begin().await?;

        // Update with new token value and rotation info
        let update_sql = adapt_sql(
            is_postgres,
            "UPDATE router_tokens SET \
             token = ?, \
             key_version = ?, \
             old_key_hash = ?, \
             old_key_expires_at = ?, \
             last_rotated_at = ? \
             WHERE token = ?",
        );

        sqlx::query(&update_sql)
            .bind(&new_token)
            .bind(new_version)
            .bind(&old_key_hash)
            .bind(transition_ends_at)
            .bind(now)
            .bind(token)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(TokenRotationResult {
            new_token,
            key_prefix: prefix,
            key_version: new_version,
            transition_ends_at,
            old_key_version: old_version,
        })
    }

    /// Validate a token, also checking old key during transition period
    pub async fn validate(db: &Database, token: &str) -> Result<Option<RouterToken>> {
        let conn = db.get_connection()?;
        let now = Self::current_timestamp();

        // First, try to find the token directly
        let sql = adapt_sql(
            db.kind() == "postgres",
            "SELECT token, user_id, status, quota_limit, used_quota, expired_time, accessed_time, \
             key_version, old_key_hash, old_key_expires_at, ip_whitelist, key_prefix, created_at, last_rotated_at \
             FROM router_tokens WHERE token = ? AND status = 'active'",
        );

        if let Some(t) = sqlx::query_as::<_, RouterToken>(&sql)
            .bind(token)
            .fetch_optional(conn.pool())
            .await?
        {
            // Check if token is expired
            if t.expired_time > 0 && now > t.expired_time {
                return Ok(None);
            }
            return Ok(Some(t));
        }

        // Token not found directly - check if it's an old key during transition period
        let token_hash = format!("{:x}", md5::compute(token.as_bytes()));

        let old_key_sql = adapt_sql(
            db.kind() == "postgres",
            "SELECT token, user_id, status, quota_limit, used_quota, expired_time, accessed_time, \
             key_version, old_key_hash, old_key_expires_at, ip_whitelist, key_prefix, created_at, last_rotated_at \
             FROM router_tokens WHERE old_key_hash = ? AND old_key_expires_at > ? AND status = 'active'",
        );

        if let Some(t) = sqlx::query_as::<_, RouterToken>(&old_key_sql)
            .bind(&token_hash)
            .bind(now)
            .fetch_optional(conn.pool())
            .await?
        {
            // Old key is valid during transition period
            // Check if token itself is expired
            if t.expired_time > 0 && now > t.expired_time {
                return Ok(None);
            }
            return Ok(Some(t));
        }

        Ok(None)
    }

    /// Validates a token and returns detailed result distinguishing between invalid and expired
    pub async fn validate_detailed(
        db: &Database,
        token: &str,
    ) -> Result<RouterTokenValidationResult> {
        let conn = db.get_connection()?;
        let now = Self::current_timestamp();

        // First, try to find the token directly
        let sql = adapt_sql(
            db.kind() == "postgres",
            "SELECT token, user_id, status, quota_limit, used_quota, expired_time, accessed_time, \
             key_version, old_key_hash, old_key_expires_at, ip_whitelist, key_prefix, created_at, last_rotated_at \
             FROM router_tokens WHERE token = ? AND status = 'active'",
        );

        if let Some(t) = sqlx::query_as::<_, RouterToken>(&sql)
            .bind(token)
            .fetch_optional(conn.pool())
            .await?
        {
            // Check if token is expired
            if t.expired_time > 0 && now > t.expired_time {
                return Ok(RouterTokenValidationResult::Expired);
            }
            return Ok(RouterTokenValidationResult::Valid(t));
        }

        // Check if it's an old key during transition period
        let token_hash = format!("{:x}", md5::compute(token.as_bytes()));

        let old_key_sql = adapt_sql(
            db.kind() == "postgres",
            "SELECT token, user_id, status, quota_limit, used_quota, expired_time, accessed_time, \
             key_version, old_key_hash, old_key_expires_at, ip_whitelist, key_prefix, created_at, last_rotated_at \
             FROM router_tokens WHERE old_key_hash = ? AND old_key_expires_at > ? AND status = 'active'",
        );

        if let Some(t) = sqlx::query_as::<_, RouterToken>(&old_key_sql)
            .bind(&token_hash)
            .bind(now)
            .fetch_optional(conn.pool())
            .await?
        {
            // Old key is valid during transition period
            if t.expired_time > 0 && now > t.expired_time {
                return Ok(RouterTokenValidationResult::Expired);
            }
            return Ok(RouterTokenValidationResult::Valid(t));
        }

        Ok(RouterTokenValidationResult::Invalid)
    }

    /// Update the accessed_time for a token (non-blocking, best-effort)
    pub async fn update_accessed_time(db: &Database, token: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let now = Self::current_timestamp();

        let sql = adapt_sql(
            db.kind() == "postgres",
            "UPDATE router_tokens SET accessed_time = ? WHERE token = ?",
        );

        sqlx::query(&sql)
            .bind(now)
            .bind(token)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    /// Check if quota is sufficient without deducting.
    /// Cost parameter uses i64 nanodollars for precision.
    /// quota_limit = -1 means unlimited.
    pub async fn check_quota(db: &Database, token: &str, cost: i64) -> Result<bool> {
        let conn = db.get_connection()?;

        if cost <= 0 {
            return Ok(true);
        }

        let quota_sql = adapt_sql(
            db.kind() == "postgres",
            "SELECT quota_limit, used_quota FROM router_tokens WHERE token = ?",
        );
        let token_quota: Option<(i64, i64)> = sqlx::query_as(&quota_sql)
            .bind(token)
            .fetch_optional(conn.pool())
            .await?;

        if let Some((quota_limit, used_quota)) = token_quota {
            // quota_limit = -1 means unlimited
            if quota_limit >= 0 && used_quota + cost > quota_limit {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Deduct quota from token atomically.
    /// Returns Ok(true) if deduction successful, Ok(false) if insufficient quota.
    /// Cost parameter uses i64 nanodollars for precision.
    /// quota_limit = -1 means unlimited.
    pub async fn deduct_quota(db: &Database, token: &str, cost: i64) -> Result<bool> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        if cost <= 0 {
            return Ok(true);
        }

        let mut tx = conn.pool().begin().await?;

        // Read quota_limit and used_quota in one query. Any DB failure propagates as an error.
        let quota_sql = adapt_sql(
            is_postgres,
            "SELECT quota_limit, used_quota FROM router_tokens WHERE token = ?",
        );
        let token_quota: Option<(i64, i64)> = sqlx::query_as(&quota_sql)
            .bind(token)
            .fetch_optional(&mut *tx)
            .await?;

        if let Some((quota_limit, used_quota)) = token_quota {
            // quota_limit = -1 means unlimited
            if quota_limit >= 0 && used_quota + cost > quota_limit {
                tx.rollback().await?;
                return Ok(false);
            }
        }

        let deduct_sql = adapt_sql(
            is_postgres,
            "UPDATE router_tokens SET used_quota = used_quota + ? WHERE token = ?",
        );
        sqlx::query(&deduct_sql)
            .bind(cost)
            .bind(token)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(true)
    }

    /// Revoke old key version immediately
    pub async fn revoke_old_key(db: &Database, token: &str) -> Result<bool> {
        let conn = db.get_connection()?;
        let sql = adapt_sql(
            db.kind() == "postgres",
            "UPDATE router_tokens SET old_key_hash = NULL, old_key_expires_at = 0 WHERE token = ?",
        );
        let result = sqlx::query(&sql).bind(token).execute(conn.pool()).await?;
        Ok(result.rows_affected() > 0)
    }

    /// Set IP whitelist for a token
    pub async fn set_ip_whitelist(db: &Database, token: &str, ip_whitelist: &str) -> Result<bool> {
        let conn = db.get_connection()?;
        let sql = adapt_sql(
            db.kind() == "postgres",
            "UPDATE router_tokens SET ip_whitelist = ? WHERE token = ?",
        );
        let result = sqlx::query(&sql)
            .bind(ip_whitelist)
            .bind(token)
            .execute(conn.pool())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Check if IP is allowed for token
    pub async fn is_ip_allowed(db: &Database, token: &str, client_ip: &str) -> Result<bool> {
        let conn = db.get_connection()?;
        let sql = adapt_sql(
            db.kind() == "postgres",
            "SELECT ip_whitelist FROM router_tokens WHERE token = ? AND status = 'active'",
        );

        let ip_whitelist: Option<String> = sqlx::query_scalar(&sql)
            .bind(token)
            .fetch_optional(conn.pool())
            .await?;

        match ip_whitelist {
            None => Ok(true),                              // No whitelist = all IPs allowed
            Some(ref list) if list.is_empty() => Ok(true), // Empty whitelist = all IPs allowed
            Some(list) => {
                // Parse whitelist (comma-separated CIDR ranges or IPs)
                for entry in list.split(',') {
                    let entry = entry.trim();
                    if entry.is_empty() {
                        continue;
                    }
                    // Simple exact match for now (CIDR matching would require ipnet crate)
                    if entry == client_ip {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }
}

/// Repository wrapper that implements the standard [`CrudRepository`] contract for tokens.
///
/// The token string itself serves as the record ID.
/// `update` replaces the full token record: delete + re-insert with the caller-provided
/// `id` as the token value, keeping the rest of `input` intact.
pub struct RouterTokenRepository<'a>(pub &'a Database);

#[async_trait::async_trait]
impl<'a> CrudRepository<RouterToken, String, DatabaseError> for RouterTokenRepository<'a> {
    async fn find_by_id(&self, id: &String) -> Result<Option<RouterToken>> {
        RouterTokenModel::find_by_token(self.0, id).await
    }

    async fn list(&self) -> Result<Vec<RouterToken>> {
        RouterTokenModel::list(self.0).await
    }

    async fn create(&self, input: &RouterToken) -> Result<RouterToken> {
        RouterTokenModel::create(self.0, input).await?;
        self.find_by_id(&input.token)
            .await?
            .ok_or_else(|| DatabaseError::Query("token disappeared after insert".to_string()))
    }

    async fn update(&self, id: &String, input: &RouterToken) -> Result<bool> {
        let exists = self.find_by_id(id).await?.is_some();
        if !exists {
            return Ok(false);
        }
        // Delete old token, then insert the new record with the canonical id.
        RouterTokenModel::delete(self.0, id).await?;
        let mut record = input.clone();
        record.token = id.clone();
        RouterTokenModel::create(self.0, &record).await?;
        Ok(true)
    }

    async fn delete(&self, id: &String) -> Result<bool> {
        let exists = self.find_by_id(id).await?.is_some();
        if exists {
            RouterTokenModel::delete(self.0, id).await?;
        }
        Ok(exists)
    }
}
