//! Database operations for token management
//!
//! This crate handles all database operations related to API tokens,
//! including validation, quota tracking, and CRUD operations.

use burncloud_database::{Database, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Token validation result that distinguishes between invalid and expired tokens
#[derive(Debug, Clone)]
pub enum TokenValidationResult {
    Valid(DbToken),
    Invalid,
    Expired,
}

/// API Token configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbToken {
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
}

pub struct TokenModel;

impl TokenModel {
    /// List all tokens
    pub async fn list(db: &Database) -> Result<Vec<DbToken>> {
        let conn = db.get_connection()?;
        let tokens = sqlx::query_as::<_, DbToken>(
            "SELECT token, user_id, status, quota_limit, used_quota, expired_time, accessed_time FROM router_tokens",
        )
        .fetch_all(conn.pool())
        .await?;
        Ok(tokens)
    }

    /// Create a new token
    pub async fn create(db: &Database, t: &DbToken) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let placeholders = if is_postgres {
            "$1, $2, $3, $4, $5, $6, $7"
        } else {
            "?, ?, ?, ?, ?, ?, ?"
        };
        let sql = format!(
            "INSERT INTO router_tokens (token, user_id, status, quota_limit, used_quota, expired_time, accessed_time) VALUES ({})",
            placeholders
        );
        sqlx::query(&sql)
            .bind(&t.token)
            .bind(&t.user_id)
            .bind(&t.status)
            .bind(t.quota_limit)
            .bind(t.used_quota)
            .bind(t.expired_time)
            .bind(t.accessed_time)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    /// Delete a token
    pub async fn delete(db: &Database, token: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "DELETE FROM router_tokens WHERE token = $1"
        } else {
            "DELETE FROM router_tokens WHERE token = ?"
        };
        sqlx::query(sql).bind(token).execute(conn.pool()).await?;
        Ok(())
    }

    /// Update token status
    pub async fn update_status(db: &Database, token: &str, status: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "UPDATE router_tokens SET status = $1 WHERE token = $2"
        } else {
            "UPDATE router_tokens SET status = ? WHERE token = ?"
        };
        sqlx::query(sql)
            .bind(status)
            .bind(token)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    /// Validate a token and return the token data if valid
    pub async fn validate(db: &Database, token: &str) -> Result<Option<DbToken>> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "SELECT token, user_id, status, quota_limit, used_quota, expired_time, accessed_time FROM router_tokens WHERE token = $1 AND status = 'active'"
        } else {
            "SELECT token, user_id, status, quota_limit, used_quota, expired_time, accessed_time FROM router_tokens WHERE token = ? AND status = 'active'"
        };
        let token_data = sqlx::query_as::<_, DbToken>(sql)
            .bind(token)
            .fetch_optional(conn.pool())
            .await?;

        // Check if token is expired
        if let Some(ref t) = token_data {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            // expired_time > 0 means it has an expiration time
            // If current time exceeds expiration, token is expired
            if t.expired_time > 0 && now > t.expired_time {
                return Ok(None); // Token expired
            }
        }

        Ok(token_data)
    }

    /// Validates a token and returns detailed result distinguishing between invalid and expired
    pub async fn validate_detailed(db: &Database, token: &str) -> Result<TokenValidationResult> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "SELECT token, user_id, status, quota_limit, used_quota, expired_time, accessed_time FROM router_tokens WHERE token = $1 AND status = 'active'"
        } else {
            "SELECT token, user_id, status, quota_limit, used_quota, expired_time, accessed_time FROM router_tokens WHERE token = ? AND status = 'active'"
        };
        let token_data = sqlx::query_as::<_, DbToken>(sql)
            .bind(token)
            .fetch_optional(conn.pool())
            .await?;

        match token_data {
            Some(t) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;

                // expired_time > 0 means it has an expiration time
                // If current time exceeds expiration, token is expired
                if t.expired_time > 0 && now > t.expired_time {
                    Ok(TokenValidationResult::Expired)
                } else {
                    Ok(TokenValidationResult::Valid(t))
                }
            }
            None => Ok(TokenValidationResult::Invalid),
        }
    }

    /// Update the accessed_time for a token (non-blocking, best-effort)
    pub async fn update_accessed_time(db: &Database, token: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let sql = if db.kind() == "postgres" {
            "UPDATE router_tokens SET accessed_time = $1 WHERE token = $2"
        } else {
            "UPDATE router_tokens SET accessed_time = ? WHERE token = ?"
        };

        sqlx::query(sql)
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

        let quota_sql = if db.kind() == "postgres" {
            "SELECT quota_limit, used_quota FROM router_tokens WHERE token = $1"
        } else {
            "SELECT quota_limit, used_quota FROM router_tokens WHERE token = ?"
        };
        let token_quota: Option<(i64, i64)> = sqlx::query_as(quota_sql)
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
        let quota_sql = if is_postgres {
            "SELECT quota_limit, used_quota FROM router_tokens WHERE token = $1"
        } else {
            "SELECT quota_limit, used_quota FROM router_tokens WHERE token = ?"
        };
        let token_quota: Option<(i64, i64)> = sqlx::query_as(quota_sql)
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

        let deduct_sql = if is_postgres {
            "UPDATE router_tokens SET used_quota = used_quota + $1 WHERE token = $2"
        } else {
            "UPDATE router_tokens SET used_quota = used_quota + ? WHERE token = ?"
        };
        sqlx::query(deduct_sql)
            .bind(cost)
            .bind(token)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(true)
    }
}
