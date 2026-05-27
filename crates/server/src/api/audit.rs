//! API endpoints for audit logging.
//!
//! Provides endpoints for querying and exporting audit logs.

use crate::api::response::{err, ok};
use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use axum::Router;
use burncloud_database_audit::AuditLogQuery;
use burncloud_service_audit::AuditService;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// Start time filter (ISO 8601)
    pub start_time: Option<String>,
    /// End time filter (ISO 8601)
    pub end_time: Option<String>,
    /// Filter by actor ID
    pub actor_id: Option<String>,
    /// Filter by action
    pub action: Option<String>,
    /// Filter by resource type
    pub resource_type: Option<String>,
    /// Filter by resource ID
    pub resource_id: Option<String>,
    /// Filter by status
    pub status: Option<String>,
    /// Maximum number of results (default: 100, max: 1000)
    pub limit: Option<i64>,
    /// Offset for pagination
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogListResponse {
    pub logs: Vec<AuditLogEntry>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct AuditLogEntry {
    pub id: i64,
    pub timestamp: String,
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

impl From<burncloud_database_audit::AuditLog> for AuditLogEntry {
    fn from(log: burncloud_database_audit::AuditLog) -> Self {
        Self {
            id: log.id,
            timestamp: log.timestamp.to_rfc3339(),
            actor_id: log.actor_id,
            actor_ip: log.actor_ip,
            action: log.action,
            resource_type: log.resource_type,
            resource_id: log.resource_id,
            changes: log.changes,
            status: log.status,
            error_message: log.error_message,
            metadata: log.metadata,
        }
    }
}

/// List audit logs with filters.
#[tracing::instrument(skip(state))]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> impl IntoResponse {
    let mut audit_query = AuditLogQuery::default();

    // Parse time filters
    if let Some(start_time) = &query.start_time {
        match DateTime::parse_from_rfc3339(start_time) {
            Ok(dt) => audit_query.start_time = Some(dt.with_timezone(&Utc)),
            Err(e) => {
                return err(format!("Invalid start_time format: {}", e)).into_response();
            }
        }
    }
    if let Some(end_time) = &query.end_time {
        match DateTime::parse_from_rfc3339(end_time) {
            Ok(dt) => audit_query.end_time = Some(dt.with_timezone(&Utc)),
            Err(e) => {
                return err(format!("Invalid end_time format: {}", e)).into_response();
            }
        }
    }

    audit_query.actor_id = query.actor_id;
    audit_query.action = query.action;
    audit_query.resource_type = query.resource_type;
    audit_query.resource_id = query.resource_id;
    audit_query.status = query.status;
    audit_query.limit = query.limit.map(|l| l.min(1000));
    audit_query.offset = query.offset;

    match AuditService::query(&state.db, audit_query.clone()).await {
        Ok(logs) => {
            let count = match AuditService::count(&state.db, audit_query).await {
                Ok(c) => c,
                Err(e) => {
                    return err(format!("Failed to count audit logs: {}", e)).into_response();
                }
            };

            let entries: Vec<AuditLogEntry> = logs.into_iter().map(|l| l.into()).collect();
            ok(AuditLogListResponse {
                logs: entries,
                total: count,
                limit: query.limit.unwrap_or(100),
                offset: query.offset.unwrap_or(0),
            }).into_response()
        }
        Err(e) => err(format!("Failed to query audit logs: {}", e)).into_response(),
    }
}

/// Get a single audit log by ID.
#[tracing::instrument(skip(state))]
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match AuditService::get_by_id(&state.db, id).await {
        Ok(Some(log)) => ok(AuditLogEntry::from(log)).into_response(),
        Ok(None) => err("Audit log not found").into_response(),
        Err(e) => err(format!("Failed to get audit log: {}", e)).into_response(),
    }
}

/// Export audit logs as CSV.
#[tracing::instrument(skip(state))]
pub async fn export(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Response {
    let mut audit_query = AuditLogQuery::default();

    // Parse time filters
    if let Some(start_time) = &query.start_time {
        match DateTime::parse_from_rfc3339(start_time) {
            Ok(dt) => audit_query.start_time = Some(dt.with_timezone(&Utc)),
            Err(_) => {
                return err("Invalid start_time format").into_response();
            }
        }
    }
    if let Some(end_time) = &query.end_time {
        match DateTime::parse_from_rfc3339(end_time) {
            Ok(dt) => audit_query.end_time = Some(dt.with_timezone(&Utc)),
            Err(_) => {
                return err("Invalid end_time format").into_response();
            }
        }
    }

    audit_query.actor_id = query.actor_id;
    audit_query.action = query.action;
    audit_query.resource_type = query.resource_type;
    audit_query.resource_id = query.resource_id;
    audit_query.status = query.status;
    // Max 10000 for export
    audit_query.limit = query.limit.map(|l| l.min(10000)).or(Some(10000));

    match AuditService::query(&state.db, audit_query).await {
        Ok(logs) => {
            let mut csv = String::from("id,timestamp,actor_id,actor_ip,action,resource_type,resource_id,status,error_message,changes,metadata\n");
            
            for log in logs {
                csv.push_str(&format!(
                    "{},{},{},{},{},{},{},{},{},{},{}\n",
                    log.id,
                    log.timestamp.to_rfc3339(),
                    log.actor_id,
                    log.actor_ip,
                    log.action,
                    log.resource_type,
                    log.resource_id,
                    log.status,
                    log.error_message.unwrap_or_default(),
                    log.changes.map(|c| c.to_string()).unwrap_or_default(),
                    log.metadata.map(|m| m.to_string()).unwrap_or_default(),
                ));
            }

            Response::builder()
                .status(axum::http::StatusCode::OK)
                .header(axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8")
                .header(axum::http::header::CONTENT_DISPOSITION, "attachment; filename=audit_logs.csv")
                .body(csv.into())
                .unwrap()
        }
        Err(e) => err(format!("Failed to export audit logs: {}", e)).into_response(),
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/console/api/audit-logs", axum::routing::get(list))
        .route("/console/api/audit-logs/export", axum::routing::get(export))
        .route("/console/api/audit-logs/{id}", axum::routing::get(get))
}
