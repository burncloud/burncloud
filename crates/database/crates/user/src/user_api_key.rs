use crate::common::current_timestamp;
use burncloud_database::{adapt_sql, Database, Result};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// User API key for application-level authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserApiKey {
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

// Manual FromRow: SQLite stores unlimited_quota as INTEGER (BIGINT in sqlx::Any),
// which cannot be automatically decoded into bool. Convert i64 → bool explicitly.
impl<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> for UserApiKey {
    fn from_row(row: &'r sqlx::any::AnyRow) -> std::result::Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(UserApiKey {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            key: row.try_get("key")?,
            status: row.try_get("status")?,
            name: row.try_get("name")?,
            remain_quota: row.try_get("remain_quota")?,
            unlimited_quota: row.try_get::<i64, _>("unlimited_quota")? != 0,
            used_quota: row.try_get("used_quota")?,
            created_time: row.try_get("created_time")?,
            accessed_time: row.try_get("accessed_time")?,
            expired_time: row.try_get("expired_time")?,
        })
    }
}

/// Input for creating a user API key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserApiKeyInput {
    pub user_id: String,
    pub name: Option<String>,
    pub remain_quota: Option<i64>,
    pub unlimited_quota: Option<bool>,
    pub expired_time: Option<i64>,
}

/// Input for updating a user API key
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserApiKeyUpdateInput {
    pub name: Option<String>,
    pub remain_quota: Option<i64>,
    pub status: Option<i32>,
    pub expired_time: Option<i64>,
}

pub struct UserApiKeyModel;

impl UserApiKeyModel {
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

    /// Create a new user API key
    pub async fn create(db: &Database, input: &UserApiKeyInput) -> Result<UserApiKey> {
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
            INSERT INTO user_api_keys (user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time)
            VALUES ($1, $2, 1, $3, $4, $5, 0, $6, $6, $7)
            RETURNING id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time
            "#
        } else {
            r#"
            INSERT INTO user_api_keys (user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time)
            VALUES (?, ?, 1, ?, ?, ?, 0, ?, ?, ?)
            "#
        };

        let mut tx = pool.begin().await?;

        let token = if is_postgres {
            let row = sqlx::query_as::<_, UserApiKey>(sql)
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
            UserApiKey {
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

    /// Get a user API key by key
    pub async fn get_by_key(db: &Database, key: &str) -> Result<Option<UserApiKey>> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => "SELECT id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time FROM user_api_keys WHERE key = $1",
            _ => "SELECT id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time FROM user_api_keys WHERE key = ?",
        };

        let token = sqlx::query_as(sql)
            .bind(key)
            .fetch_optional(conn.pool())
            .await?;

        Ok(token)
    }

    /// List user API keys with pagination and optional user_id filter
    pub async fn list(
        db: &Database,
        limit: i32,
        offset: i32,
        user_id: Option<&str>,
    ) -> Result<Vec<UserApiKey>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let tokens = match user_id {
            Some(uid) => {
                let sql = adapt_sql(is_postgres, "SELECT id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time FROM user_api_keys WHERE user_id = ? ORDER BY id DESC LIMIT ? OFFSET ?");
                sqlx::query_as(&sql)
                    .bind(uid)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(conn.pool())
                    .await?
            }
            None => {
                let sql = adapt_sql(is_postgres, "SELECT id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time FROM user_api_keys ORDER BY id DESC LIMIT ? OFFSET ?");
                sqlx::query_as(&sql)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(conn.pool())
                    .await?
            }
        };

        Ok(tokens)
    }

    /// Update a user API key by key
    pub async fn update(db: &Database, key: &str, input: &UserApiKeyUpdateInput) -> Result<bool> {
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
            "UPDATE user_api_keys SET {} WHERE key = {}",
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

    /// Delete a user API key by key
    pub async fn delete(db: &Database, key: &str) -> Result<bool> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => "DELETE FROM user_api_keys WHERE key = $1",
            _ => "DELETE FROM user_api_keys WHERE key = ?",
        };

        let result = sqlx::query(sql).bind(key).execute(conn.pool()).await?;

        Ok(result.rows_affected() > 0)
    }
}
