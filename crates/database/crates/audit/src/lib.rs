//! Database operations for audit logging.
//!
//! This module provides audit log storage and retrieval for sensitive operations
//! including authentication, configuration changes, and business operations.

use burncloud_database::{Database, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

/// Audit log entry representing a single auditable event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    /// Unique identifier for the audit log entry
    pub id: i64,
    /// ISO 8601 timestamp of the operation
    pub timestamp: DateTime<Utc>,
    /// ID of the user who performed the action
    pub actor_id: String,
    /// IP address of the actor
    pub actor_ip: String,
    /// Type of action performed (e.g., "user.login", "channel.create")
    pub action: String,
    /// Type of resource affected (e.g., "user", "channel", "token")
    pub resource_type: String,
    /// ID of the affected resource
    pub resource_id: String,
    /// JSON representation of the changes (before/after for updates)
    pub changes: Option<serde_json::Value>,
    /// Result of the operation: "success" or "failure"
    pub status: String,
    /// Optional error message for failed operations
    pub error_message: Option<String>,
    /// Additional metadata as JSON
    pub metadata: Option<serde_json::Value>,
}

/// Input for creating a new audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogInput {
    pub actor_id: String,
    pub actor_ip: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub changes: Option<serde_json::Value>,
    pub status: String,
    pub error_message: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Query parameters for filtering audit logs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditLogQuery {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub actor_id: Option<String>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub struct AuditDatabase;

impl AuditDatabase {
    /// Initialize the audit_logs table.
    pub async fn init(db: &Database) -> Result<()> {
        let conn = db.get_connection()?;
        let kind = db.kind();

        let create_table_sql = match kind.as_str() {
            "sqlite" => r#"
                CREATE TABLE IF NOT EXISTS audit_logs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
                    actor_id TEXT NOT NULL,
                    actor_ip TEXT NOT NULL,
                    action TEXT NOT NULL,
                    resource_type TEXT NOT NULL,
                    resource_id TEXT NOT NULL,
                    changes TEXT,
                    status TEXT NOT NULL DEFAULT 'success',
                    error_message TEXT,
                    metadata TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp);
                CREATE INDEX IF NOT EXISTS idx_audit_logs_actor_id ON audit_logs(actor_id);
                CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
                CREATE INDEX IF NOT EXISTS idx_audit_logs_resource_type ON audit_logs(resource_type);
                CREATE INDEX IF NOT EXISTS idx_audit_logs_resource_id ON audit_logs(resource_id);
            "#,
            "postgres" => r#"
                CREATE TABLE IF NOT EXISTS audit_logs (
                    id SERIAL PRIMARY KEY,
                    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    actor_id TEXT NOT NULL,
                    actor_ip TEXT NOT NULL,
                    action TEXT NOT NULL,
                    resource_type TEXT NOT NULL,
                    resource_id TEXT NOT NULL,
                    changes JSONB,
                    status TEXT NOT NULL DEFAULT 'success',
                    error_message TEXT,
                    metadata JSONB
                );
                CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp);
                CREATE INDEX IF NOT EXISTS idx_audit_logs_actor_id ON audit_logs(actor_id);
                CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
                CREATE INDEX IF NOT EXISTS idx_audit_logs_resource_type ON audit_logs(resource_type);
                CREATE INDEX IF NOT EXISTS idx_audit_logs_resource_id ON audit_logs(resource_id);
            "#,
            _ => unreachable!("Unsupported database kind"),
        };

        sqlx::query(create_table_sql)
            .execute(conn.pool())
            .await?;
        
        tracing::info!("AuditDatabase: audit_logs table created/verified.");
        Ok(())
    }

    /// Create a new audit log entry.
    pub async fn create(db: &Database, input: AuditLogInput) -> Result<i64> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let changes_json = input.changes.as_ref().map(|v| v.to_string());
        let metadata_json = input.metadata.as_ref().map(|v| v.to_string());

        let sql = format!(
            r#"
            INSERT INTO audit_logs (actor_id, actor_ip, action, resource_type, resource_id, changes, status, error_message, metadata)
            VALUES ({ph1}, {ph2}, {ph3}, {ph4}, {ph5}, {ph6}, {ph7}, {ph8}, {ph9})
            "#,
            ph1 = burncloud_database::ph(is_postgres, 1),
            ph2 = burncloud_database::ph(is_postgres, 2),
            ph3 = burncloud_database::ph(is_postgres, 3),
            ph4 = burncloud_database::ph(is_postgres, 4),
            ph5 = burncloud_database::ph(is_postgres, 5),
            ph6 = if changes_json.is_some() { burncloud_database::ph(is_postgres, 6) } else { "NULL".to_string() },
            ph7 = burncloud_database::ph(is_postgres, 7),
            ph8 = if input.error_message.is_some() { burncloud_database::ph(is_postgres, 8) } else { "NULL".to_string() },
            ph9 = if metadata_json.is_some() { burncloud_database::ph(is_postgres, 9) } else { "NULL".to_string() },
        );

        let mut query = sqlx::query(&sql)
            .bind(&input.actor_id)
            .bind(&input.actor_ip)
            .bind(&input.action)
            .bind(&input.resource_type)
            .bind(&input.resource_id);
        
        if let Some(changes) = changes_json {
            query = query.bind(changes);
        }
        
        query = query.bind(&input.status);
        
        if let Some(error_msg) = &input.error_message {
            query = query.bind(error_msg);
        }
        
        if let Some(metadata) = metadata_json {
            query = query.bind(metadata);
        }

        let result = query.execute(conn.pool()).await?;

        Ok(result.last_insert_id().unwrap_or(0))
    }

    /// Query audit logs with filters.
    pub async fn query(db: &Database, params: AuditLogQuery) -> Result<Vec<AuditLog>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let mut conditions = Vec::new();
        let mut bind_values: Vec<String> = Vec::new();
        let mut bind_index = 1;
        
        if let Some(start_time) = params.start_time {
            conditions.push(format!("timestamp >= {}", burncloud_database::ph(is_postgres, bind_index)));
            bind_values.push(start_time.to_rfc3339());
            bind_index += 1;
        }
        if let Some(end_time) = params.end_time {
            conditions.push(format!("timestamp <= {}", burncloud_database::ph(is_postgres, bind_index)));
            bind_values.push(end_time.to_rfc3339());
            bind_index += 1;
        }
        if let Some(actor_id) = &params.actor_id {
            conditions.push(format!("actor_id = {}", burncloud_database::ph(is_postgres, bind_index)));
            bind_values.push(actor_id.clone());
            bind_index += 1;
        }
        if let Some(action) = &params.action {
            conditions.push(format!("action = {}", burncloud_database::ph(is_postgres, bind_index)));
            bind_values.push(action.clone());
            bind_index += 1;
        }
        if let Some(resource_type) = &params.resource_type {
            conditions.push(format!("resource_type = {}", burncloud_database::ph(is_postgres, bind_index)));
            bind_values.push(resource_type.clone());
            bind_index += 1;
        }
        if let Some(resource_id) = &params.resource_id {
            conditions.push(format!("resource_id = {}", burncloud_database::ph(is_postgres, bind_index)));
            bind_values.push(resource_id.clone());
            bind_index += 1;
        }
        if let Some(status) = &params.status {
            conditions.push(format!("status = {}", burncloud_database::ph(is_postgres, bind_index)));
            bind_values.push(status.clone());
            bind_index += 1;
        }

        let _ = bind_index; // suppress unused warning
        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let limit = params.limit.unwrap_or(100).min(1000);
        let offset = params.offset.unwrap_or(0);

        let sql = format!(
            "SELECT id, timestamp, actor_id, actor_ip, action, resource_type, resource_id, changes, status, error_message, metadata FROM audit_logs {} ORDER BY timestamp DESC LIMIT {} OFFSET {}",
            where_clause, limit, offset
        );

        let mut query = sqlx::query(&sql);
        for value in bind_values {
            query = query.bind(value);
        }

        let rows = query.fetch_all(conn.pool()).await?;

        let mut logs = Vec::new();
        for row in rows {
            let changes_str: Option<String> = row.try_get("changes").ok();
            let metadata_str: Option<String> = row.try_get("metadata").ok();
            
            let timestamp_str: String = row.try_get("timestamp")?;
            let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            logs.push(AuditLog {
                id: row.try_get("id")?,
                timestamp,
                actor_id: row.try_get("actor_id")?,
                actor_ip: row.try_get("actor_ip")?,
                action: row.try_get("action")?,
                resource_type: row.try_get("resource_type")?,
                resource_id: row.try_get("resource_id")?,
                changes: changes_str.and_then(|s| serde_json::from_str(&s).ok()),
                status: row.try_get("status")?,
                error_message: row.try_get("error_message")?,
                metadata: metadata_str.and_then(|s| serde_json::from_str(&s).ok()),
            });
        }

        Ok(logs)
    }

    /// Get a single audit log by ID.
    pub async fn get_by_id(db: &Database, id: i64) -> Result<Option<AuditLog>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = format!(
            "SELECT id, timestamp, actor_id, actor_ip, action, resource_type, resource_id, changes, status, error_message, metadata FROM audit_logs WHERE id = {}",
            burncloud_database::ph(is_postgres, 1)
        );

        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(conn.pool())
            .await?;

        match row {
            Some(row) => {
                let changes_str: Option<String> = row.try_get("changes").ok();
                let metadata_str: Option<String> = row.try_get("metadata").ok();
                
                let timestamp_str: String = row.try_get("timestamp")?;
                let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(Some(AuditLog {
                    id: row.try_get("id")?,
                    timestamp,
                    actor_id: row.try_get("actor_id")?,
                    actor_ip: row.try_get("actor_ip")?,
                    action: row.try_get("action")?,
                    resource_type: row.try_get("resource_type")?,
                    resource_id: row.try_get("resource_id")?,
                    changes: changes_str.and_then(|s| serde_json::from_str(&s).ok()),
                    status: row.try_get("status")?,
                    error_message: row.try_get("error_message")?,
                    metadata: metadata_str.and_then(|s| serde_json::from_str(&s).ok()),
                }))
            }
            None => Ok(None),
        }
    }

    /// Count audit logs matching the query.
    pub async fn count(db: &Database, params: AuditLogQuery) -> Result<i64> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let mut conditions = Vec::new();
        let mut bind_values: Vec<String> = Vec::new();
        let mut bind_index = 1;
        
        if let Some(start_time) = params.start_time {
            conditions.push(format!("timestamp >= {}", burncloud_database::ph(is_postgres, bind_index)));
            bind_values.push(start_time.to_rfc3339());
            bind_index += 1;
        }
        if let Some(end_time) = params.end_time {
            conditions.push(format!("timestamp <= {}", burncloud_database::ph(is_postgres, bind_index)));
            bind_values.push(end_time.to_rfc3339());
            bind_index += 1;
        }
        if let Some(actor_id) = &params.actor_id {
            conditions.push(format!("actor_id = {}", burncloud_database::ph(is_postgres, bind_index)));
            bind_values.push(actor_id.clone());
            bind_index += 1;
        }
        if let Some(action) = &params.action {
            conditions.push(format!("action = {}", burncloud_database::ph(is_postgres, bind_index)));
            bind_values.push(action.clone());
            bind_index += 1;
        }
        if let Some(resource_type) = &params.resource_type {
            conditions.push(format!("resource_type = {}", burncloud_database::ph(is_postgres, bind_index)));
            bind_values.push(resource_type.clone());
            bind_index += 1;
        }
        if let Some(resource_id) = &params.resource_id {
            conditions.push(format!("resource_id = {}", burncloud_database::ph(is_postgres, bind_index)));
            bind_values.push(resource_id.clone());
            bind_index += 1;
        }
        if let Some(status) = &params.status {
            conditions.push(format!("status = {}", burncloud_database::ph(is_postgres, bind_index)));
            bind_values.push(status.clone());
            bind_index += 1;
        }

        let _ = bind_index; // suppress unused warning
        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!("SELECT COUNT(*) as count FROM audit_logs {}", where_clause);

        let mut query = sqlx::query(&sql);
        for value in bind_values {
            query = query.bind(value);
        }

        let row = query.fetch_one(conn.pool()).await?;

        Ok(row.try_get("count")?)
    }
}
