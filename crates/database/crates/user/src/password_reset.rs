use burncloud_database::{Database, Result};
use sqlx::Row;

pub struct PasswordResetToken {
    pub token: String,
    pub user_id: String,
    pub expires_at: String,
    pub used_at: Option<String>,
}

pub struct PasswordResetDatabase;

impl PasswordResetDatabase {
    pub async fn create_token(db: &Database, token: &str, user_id: &str, expires_at: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "INSERT INTO password_reset_tokens (token, user_id, expires_at) VALUES ($1, $2, $3)"
        } else {
            "INSERT INTO password_reset_tokens (token, user_id, expires_at) VALUES (?, ?, ?)"
        };
        sqlx::query(sql)
            .bind(token)
            .bind(user_id)
            .bind(expires_at)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    pub async fn get_token(db: &Database, token: &str) -> Result<Option<PasswordResetToken>> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "SELECT token, user_id, expires_at, used_at FROM password_reset_tokens WHERE token = $1"
        } else {
            "SELECT token, user_id, expires_at, used_at FROM password_reset_tokens WHERE token = ?"
        };
        let row = sqlx::query(sql)
            .bind(token)
            .fetch_optional(conn.pool())
            .await?;
        Ok(row.map(|r| PasswordResetToken {
            token: r.get(0),
            user_id: r.get(1),
            expires_at: r.get(2),
            used_at: r.get(3),
        }))
    }

    pub async fn mark_used(db: &Database, token: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "UPDATE password_reset_tokens SET used_at = NOW() WHERE token = $1"
        } else {
            "UPDATE password_reset_tokens SET used_at = datetime('now') WHERE token = ?"
        };
        sqlx::query(sql)
            .bind(token)
            .execute(conn.pool())
            .await?;
        Ok(())
    }
}
