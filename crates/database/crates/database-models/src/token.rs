use crate::common::current_timestamp;
use burncloud_database::{Database, Result};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Token for API authentication
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Token {
    pub id: i32,
    pub user_id: String,
    pub key: String,
    pub status: i32,
    pub name: Option<String>,
    pub remain_quota: i64,
    pub unlimited_quota: bool,
    pub used_quota: i64,
    pub created_time: Option<i64>,
    pub accessed_time: Option<i64>,
    pub expired_time: i64,
}

/// Input for creating a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInput {
    pub user_id: String,
    pub name: Option<String>,
    pub remain_quota: Option<i64>,
    pub unlimited_quota: Option<bool>,
    pub expired_time: Option<i64>,
}

/// Input for updating a token
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUpdateInput {
    pub name: Option<String>,
    pub remain_quota: Option<i64>,
    pub status: Option<i32>,
    pub expired_time: Option<i64>,
}

pub struct TokenModel;

impl TokenModel {
    /// Generate a unique token key with sk- prefix and 48 random characters
    fn generate_key() -> String {
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut rng = rand::thread_rng();
        let key: String = (0..48)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        format!("sk-{}", key)
    }

    /// Create a new token
    pub async fn create(db: &Database, input: &TokenInput) -> Result<Token> {
        let conn = db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = db.kind() == "postgres";

        let key = Self::generate_key();
        let now = current_timestamp();

        let remain_quota = input.remain_quota.unwrap_or(0);
        let unlimited_quota = input.unlimited_quota.unwrap_or(false);
        let expired_time = input.expired_time.unwrap_or(-1);

        let sql = if is_postgres {
            r#"
            INSERT INTO tokens (user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time)
            VALUES ($1, $2, 1, $3, $4, $5, 0, $6, $6, $7)
            RETURNING id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time
            "#
        } else {
            r#"
            INSERT INTO tokens (user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time)
            VALUES (?, ?, 1, ?, ?, ?, 0, ?, ?, ?)
            "#
        };

        let mut tx = pool.begin().await?;

        let token = if is_postgres {
            let row = sqlx::query_as::<_, Token>(sql)
                .bind(&input.user_id)
                .bind(&key)
                .bind(&input.name)
                .bind(remain_quota)
                .bind(unlimited_quota)
                .bind(now)
                .bind(expired_time)
                .fetch_one(&mut *tx)
                .await?;
            tx.commit().await?;
            row
        } else {
            sqlx::query(sql)
                .bind(&input.user_id)
                .bind(&key)
                .bind(&input.name)
                .bind(remain_quota)
                .bind(unlimited_quota)
                .bind(now)
                .bind(now)
                .bind(expired_time)
                .execute(&mut *tx)
                .await?;
            let id: (i64,) = sqlx::query_as("SELECT last_insert_rowid()")
                .fetch_one(&mut *tx)
                .await?;
            tx.commit().await?;
            Token {
                id: id.0 as i32,
                user_id: input.user_id.clone(),
                key,
                status: 1,
                name: input.name.clone(),
                remain_quota,
                unlimited_quota,
                used_quota: 0,
                created_time: Some(now),
                accessed_time: Some(now),
                expired_time,
            }
        };

        Ok(token)
    }

    /// Get a token by key
    pub async fn get_by_key(db: &Database, key: &str) -> Result<Option<Token>> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => "SELECT id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time FROM tokens WHERE key = $1",
            _ => "SELECT id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time FROM tokens WHERE key = ?",
        };

        let token = sqlx::query_as(sql)
            .bind(key)
            .fetch_optional(conn.pool())
            .await?;

        Ok(token)
    }

    /// List tokens with pagination and optional user_id filter
    pub async fn list(
        db: &Database,
        limit: i32,
        offset: i32,
        user_id: Option<&str>,
    ) -> Result<Vec<Token>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let tokens = match user_id {
            Some(uid) => {
                let sql = if is_postgres {
                    "SELECT id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time FROM tokens WHERE user_id = $1 ORDER BY id DESC LIMIT $2 OFFSET $3"
                } else {
                    "SELECT id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time FROM tokens WHERE user_id = ? ORDER BY id DESC LIMIT ? OFFSET ?"
                };
                sqlx::query_as(sql)
                    .bind(uid)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(conn.pool())
                    .await?
            }
            None => {
                let sql = if is_postgres {
                    "SELECT id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time FROM tokens ORDER BY id DESC LIMIT $1 OFFSET $2"
                } else {
                    "SELECT id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time FROM tokens ORDER BY id DESC LIMIT ? OFFSET ?"
                };
                sqlx::query_as(sql)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(conn.pool())
                    .await?
            }
        };

        Ok(tokens)
    }

    /// Update a token by key
    pub async fn update(db: &Database, key: &str, input: &TokenUpdateInput) -> Result<bool> {
        let conn = db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = db.kind() == "postgres";

        // Build dynamic update query
        let mut updates = Vec::new();
        let mut param_count = 1;

        if input.name.is_some() {
            updates.push(if is_postgres {
                format!("name = ${}", param_count)
            } else {
                "name = ?".to_string()
            });
            param_count += 1;
        }
        if input.remain_quota.is_some() {
            updates.push(if is_postgres {
                format!("remain_quota = ${}", param_count)
            } else {
                "remain_quota = ?".to_string()
            });
            param_count += 1;
        }
        if input.status.is_some() {
            updates.push(if is_postgres {
                format!("status = ${}", param_count)
            } else {
                "status = ?".to_string()
            });
            param_count += 1;
        }
        if input.expired_time.is_some() {
            updates.push(if is_postgres {
                format!("expired_time = ${}", param_count)
            } else {
                "expired_time = ?".to_string()
            });
            param_count += 1;
        }

        if updates.is_empty() {
            return Ok(false);
        }

        let key_param = if is_postgres {
            format!("${}", param_count)
        } else {
            "?".to_string()
        };

        let sql = format!(
            "UPDATE tokens SET {} WHERE key = {}",
            updates.join(", "),
            key_param
        );

        let mut query = sqlx::query(&sql);
        if let Some(ref name) = input.name {
            query = query.bind(name);
        }
        if let Some(remain_quota) = input.remain_quota {
            query = query.bind(remain_quota);
        }
        if let Some(status) = input.status {
            query = query.bind(status);
        }
        if let Some(expired_time) = input.expired_time {
            query = query.bind(expired_time);
        }
        query = query.bind(key);

        let result = query.execute(pool).await?;
        Ok(result.rows_affected() > 0)
    }

    /// Delete a token by key
    pub async fn delete(db: &Database, key: &str) -> Result<bool> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => "DELETE FROM tokens WHERE key = $1",
            _ => "DELETE FROM tokens WHERE key = ?",
        };

        let result = sqlx::query(sql).bind(key).execute(conn.pool()).await?;

        Ok(result.rows_affected() > 0)
    }
}
