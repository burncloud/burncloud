//! Database operations for router logs and usage statistics
//!
//! This crate handles all database operations related to router logs,
//! usage statistics, and balance deductions.

use burncloud_database::{adapt_sql, ph, phs, Database, DatabaseError, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Router log entry
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
pub struct RouterLog {
    pub id: i64,
    pub request_id: String,
    pub user_id: Option<String>,
    pub path: String,
    pub upstream_id: Option<String>,
    pub status_code: i32,
    pub latency_ms: i64,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    #[sqlx(default)]
    /// Total cost in nanodollars (9 decimal precision)
    pub cost: i64,
    #[sqlx(default)]
    pub model: Option<String>,
    #[sqlx(default)]
    pub cache_read_tokens: i32,
    #[sqlx(default)]
    pub reasoning_tokens: i32,
    #[sqlx(default)]
    pub pricing_region: Option<String>,
    #[sqlx(default)]
    pub video_tokens: i32,
    // Per-type token counts (added in billing expansion)
    #[sqlx(default)]
    pub cache_write_tokens: i32,
    #[sqlx(default)]
    pub audio_input_tokens: i32,
    #[sqlx(default)]
    pub audio_output_tokens: i32,
    #[sqlx(default)]
    pub image_tokens: i32,
    #[sqlx(default)]
    pub embedding_tokens: i32,
    // Per-type cost breakdown in nanodollars
    #[sqlx(default)]
    pub input_cost: i64,
    #[sqlx(default)]
    pub output_cost: i64,
    #[sqlx(default)]
    pub cache_read_cost: i64,
    #[sqlx(default)]
    pub cache_write_cost: i64,
    #[sqlx(default)]
    pub audio_cost: i64,
    #[sqlx(default)]
    pub image_cost: i64,
    #[sqlx(default)]
    pub video_cost: i64,
    #[sqlx(default)]
    pub reasoning_cost: i64,
    #[sqlx(default)]
    pub embedding_cost: i64,
    // L6 Observability fields (migration 0011): which router layer made the
    // decision and what color was attached. Used by Grafana for affinity_hit /
    // shaper_reject / scorer_picked / failover_N reporting.
    #[sqlx(default)]
    pub layer_decision: Option<String>,
    #[sqlx(default)]
    pub traffic_color: Option<String>,
    // Billing observability (migration 0013): why cost=0 — "ok", "price_missing",
    // "calc_error", "no_model". NULL for pre-migration rows.
    #[sqlx(default)]
    pub cost_status: Option<String>,
    // Error classification (migration 0014): why a request failed.
    // Values: "upstream_error", "timeout", "auth_failed", "rate_limit",
    // "router_reject", or NULL for successful requests.
    #[sqlx(default)]
    pub error_type: Option<String>,
    pub created_at: Option<String>,
}

/// Usage statistics for a user over a time period
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStats {
    pub total_requests: i64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    /// Total cost in nanodollars
    pub total_cost_nano: i64,
}

/// Usage statistics grouped by model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageStats {
    pub model: String,
    pub requests: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cache_read_tokens: i64,
    pub reasoning_tokens: i64,
    /// Cost in nanodollars
    pub cost_nano: i64,
}

pub struct RouterLogModel;

impl RouterLogModel {
    /// Persist a router log without changing Identity credential spend quotas.
    ///
    /// Both the legacy model API and the service entry point use the same
    /// log-only insertion path. Credential settlement is performed separately
    /// by Identity, using the cost in nanodollars for the authenticated key.
    pub async fn insert(db: &Database, log: &RouterLog) -> Result<()> {
        crate::RouterDatabase::insert_log(db, log).await
    }

    /// Get logs with pagination
    pub async fn get(db: &Database, limit: i32, offset: i32) -> Result<Vec<RouterLog>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = format!(
            "SELECT id, request_id, user_id, path, upstream_id, status_code, latency_ms, prompt_tokens, completion_tokens, cost, model, cache_read_tokens, reasoning_tokens, pricing_region, video_tokens, cache_write_tokens, audio_input_tokens, audio_output_tokens, image_tokens, embedding_tokens, input_cost, output_cost, cache_read_cost, cache_write_cost, audio_cost, image_cost, video_cost, reasoning_cost, embedding_cost, layer_decision, traffic_color, cost_status, error_type, created_at FROM router_logs ORDER BY created_at DESC {}",
            adapt_sql(is_postgres, "LIMIT ? OFFSET ?")
        );
        let logs = sqlx::query_as::<_, RouterLog>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(conn.pool())
            .await?;
        Ok(logs)
    }

    /// Get logs with optional filtering by user_id, upstream_id (channel), and model
    pub async fn get_filtered(
        db: &Database,
        user_id: Option<&str>,
        upstream_id: Option<&str>,
        model: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<RouterLog>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let mut conditions: Vec<String> = Vec::new();
        let mut param_index = 1;

        if user_id.is_some() {
            conditions.push(format!("user_id = {}", ph(is_postgres, param_index)));
            param_index += 1;
        }
        if upstream_id.is_some() {
            conditions.push(format!("upstream_id = {}", ph(is_postgres, param_index)));
            param_index += 1;
        }
        if model.is_some() {
            conditions.push(format!("model = {}", ph(is_postgres, param_index)));
            param_index += 1;
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let limit_offset = format!(
            "LIMIT {} OFFSET {}",
            ph(is_postgres, param_index),
            ph(is_postgres, param_index + 1)
        );
        let sql = format!(
            "SELECT id, request_id, user_id, path, upstream_id, status_code, latency_ms, prompt_tokens, completion_tokens, cost, model, cache_read_tokens, reasoning_tokens, pricing_region, video_tokens, cache_write_tokens, audio_input_tokens, audio_output_tokens, image_tokens, embedding_tokens, input_cost, output_cost, cache_read_cost, cache_write_cost, audio_cost, image_cost, video_cost, reasoning_cost, embedding_cost, layer_decision, traffic_color, cost_status, error_type, created_at FROM router_logs {} ORDER BY created_at DESC {}",
            where_clause, limit_offset
        );

        let mut query = sqlx::query_as::<_, RouterLog>(&sql);

        if let Some(uid) = user_id {
            query = query.bind(uid);
        }
        if let Some(upstream) = upstream_id {
            query = query.bind(upstream);
        }
        if let Some(m) = model {
            query = query.bind(m);
        }

        let logs = query
            .bind(limit)
            .bind(offset)
            .fetch_all(conn.pool())
            .await?;
        Ok(logs)
    }

    /// Get total usage by user
    pub async fn get_usage_by_user(db: &Database, user_id: &str) -> Result<(i64, i64)> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = adapt_sql(
            is_postgres,
            "SELECT SUM(prompt_tokens), SUM(completion_tokens) FROM router_logs WHERE user_id = ?",
        );
        let row: (Option<i64>, Option<i64>) = sqlx::query_as(&sql)
            .bind(user_id)
            .fetch_one(conn.pool())
            .await?;

        Ok((row.0.unwrap_or(0), row.1.unwrap_or(0)))
    }
}

/// Row shape of the aggregate usage query in [`get_usage_stats`].
/// Aliased so the row type does not trip `clippy::type_complexity`.
type UsageTotalsRow = (Option<i64>, Option<i64>, Option<i64>, Option<i64>);

/// Get aggregated usage statistics for a user over a time period
/// Period can be: "day", "week", "month"
pub async fn get_usage_stats(db: &Database, user_id: &str, period: &str) -> Result<UsageStats> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    // Calculate time threshold based on period
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| DatabaseError::Query(format!("Time error: {}", e)))?
        .as_secs() as i64;

    let threshold = match period {
        "day" => now - 24 * 60 * 60,
        "week" => now - 7 * 24 * 60 * 60,
        _ => now - 30 * 24 * 60 * 60, // Default to month for any other input
    };

    let time_filter = if is_postgres {
        format!(
            "EXTRACT(EPOCH FROM created_at)::BIGINT >= {}",
            ph(is_postgres, 2)
        )
    } else {
        format!(
            "CAST(strftime('%s', created_at) AS BIGINT) >= {}",
            ph(is_postgres, 2)
        )
    };

    let sql = format!(
        r#"
        SELECT
            COUNT(*) as total_requests,
            COALESCE(SUM(prompt_tokens), 0) as total_prompt_tokens,
            COALESCE(SUM(completion_tokens), 0) as total_completion_tokens,
            COALESCE(SUM(cost), 0) as total_cost
        FROM router_logs
        WHERE user_id = {} AND created_at IS NOT NULL AND {}
        "#,
        ph(is_postgres, 1),
        time_filter
    );

    let row: UsageTotalsRow = sqlx::query_as(&sql)
        .bind(user_id)
        .bind(threshold)
        .fetch_one(conn.pool())
        .await?;

    Ok(UsageStats {
        total_requests: row.0.unwrap_or(0),
        total_prompt_tokens: row.1.unwrap_or(0),
        total_completion_tokens: row.2.unwrap_or(0),
        total_cost_nano: row.3.unwrap_or(0),
    })
}

/// Row shape of the per-model usage aggregation in [`get_usage_stats_by_model`].
/// Aliased so the row type does not trip `clippy::type_complexity`.
type ModelUsageRow = (String, i64, i64, i64, i64, i64, i64);

/// Get usage statistics grouped by model for a user over a time period
/// Period can be: "day", "week", "month"
pub async fn get_usage_stats_by_model(
    db: &Database,
    user_id: &str,
    period: &str,
) -> Result<Vec<ModelUsageStats>> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    // Calculate time threshold — same logic as get_usage_stats
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| DatabaseError::Query(format!("Time error: {}", e)))?
        .as_secs() as i64;

    let threshold = match period {
        "day" => now - 24 * 60 * 60,
        "week" => now - 7 * 24 * 60 * 60,
        _ => now - 30 * 24 * 60 * 60,
    };

    let time_filter = if is_postgres {
        format!(
            "EXTRACT(EPOCH FROM created_at)::BIGINT >= {}",
            ph(is_postgres, 2)
        )
    } else {
        "strftime('%s', created_at) >= CAST(? AS TEXT)".to_string()
    };

    let sql = format!(
        r#"
        SELECT
            COALESCE(model, 'Unknown') as model,
            COUNT(*) as requests,
            COALESCE(SUM(prompt_tokens), 0) as prompt_tokens,
            COALESCE(SUM(completion_tokens), 0) as completion_tokens,
            COALESCE(SUM(cache_read_tokens), 0) as cache_read_tokens,
            COALESCE(SUM(reasoning_tokens), 0) as reasoning_tokens,
            COALESCE(SUM(cost), 0) as cost
        FROM router_logs
        WHERE user_id = {} AND created_at IS NOT NULL AND {}
        GROUP BY model
        ORDER BY cost DESC
        "#,
        ph(is_postgres, 1),
        time_filter
    );

    let rows: Vec<ModelUsageRow> = sqlx::query_as(&sql)
        .bind(user_id)
        .bind(threshold.to_string())
        .fetch_all(conn.pool())
        .await?;

    Ok(rows
        .into_iter()
        .map(
            |(
                model,
                requests,
                prompt_tokens,
                completion_tokens,
                cache_read_tokens,
                reasoning_tokens,
                cost_nano,
            )| ModelUsageStats {
                model,
                requests,
                prompt_tokens,
                completion_tokens,
                cache_read_tokens,
                reasoning_tokens,
                cost_nano,
            },
        )
        .collect())
}

/// Billing summary per model for reconciliation with upstream providers (e.g. Google).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingModelSummary {
    pub model: String,
    pub requests: i64,
    pub prompt_tokens: i64,
    pub cache_read_tokens: i64,
    pub completion_tokens: i64,
    pub reasoning_tokens: i64,
    /// Cost in USD (converted from nanodollars)
    pub cost_usd: f64,
}

/// Aggregate billing summary for a time period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingSummary {
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    /// Number of requests with NULL model (pre-migration data).
    pub pre_migration_requests: i64,
    pub models: Vec<BillingModelSummary>,
    pub total_cost_usd: f64,
}

/// Row shape of the per-model billing aggregation shared by
/// [`get_billing_summary`] and [`get_billing_summary_for_user`].
/// Aliased so the row type does not trip `clippy::type_complexity`.
type BillingModelRow = (String, i64, i64, i64, i64, i64, i64);

/// Build the `created_at` date-range filter used by the billing summary queries.
///
/// SQLite stores `created_at` as TEXT (`CURRENT_TIMESTAMP` →
/// "YYYY-MM-DD HH:MM:SS"), so `strftime` extracts the date portion; PostgreSQL
/// stores it as TIMESTAMP and casts it to `date`.
///
/// `first_placeholder` is the 1-based index of the first bound date parameter,
/// because the per-user query binds `user_id` before the dates. The two boolean
/// flags report whether the start / end date is bound in the filter, so callers
/// bind the same parameters in the same order.
fn billing_date_filter(
    is_postgres: bool,
    start: Option<&str>,
    end: Option<&str>,
    first_placeholder: usize,
) -> (String, bool, bool) {
    match (start, end) {
        (Some(_), Some(_)) if is_postgres => (
            format!(
                "AND created_at::date >= {}::date AND created_at::date <= {}::date",
                ph(is_postgres, first_placeholder),
                ph(is_postgres, first_placeholder + 1)
            ),
            true,
            true,
        ),
        (Some(_), None) if is_postgres => (
            format!(
                "AND created_at::date >= {}::date",
                ph(is_postgres, first_placeholder)
            ),
            true,
            false,
        ),
        (None, Some(_)) if is_postgres => (
            format!(
                "AND created_at::date <= {}::date",
                ph(is_postgres, first_placeholder)
            ),
            false,
            true,
        ),
        (Some(_), Some(_)) => (
            format!(
                "AND strftime('%Y-%m-%d', created_at) >= {} AND strftime('%Y-%m-%d', created_at) <= {}",
                ph(is_postgres, first_placeholder),
                ph(is_postgres, first_placeholder + 1)
            ),
            true,
            true,
        ),
        (Some(_), None) => (
            format!(
                "AND strftime('%Y-%m-%d', created_at) >= {}",
                ph(is_postgres, first_placeholder)
            ),
            true,
            false,
        ),
        (None, Some(_)) => (
            format!(
                "AND strftime('%Y-%m-%d', created_at) <= {}",
                ph(is_postgres, first_placeholder)
            ),
            false,
            true,
        ),
        (None, None) => (String::new(), false, false),
    }
}

/// Fold per-model billing rows into the per-model summaries and the total
/// cost in nanodollars.
fn summarize_billing_rows(rows: Vec<BillingModelRow>) -> (Vec<BillingModelSummary>, i64) {
    let mut total_cost_nano: i64 = 0;
    let models: Vec<BillingModelSummary> = rows
        .into_iter()
        .map(
            |(
                model,
                requests,
                prompt_tokens,
                cache_read_tokens,
                completion_tokens,
                reasoning_tokens,
                cost_nano,
            )| {
                total_cost_nano = total_cost_nano.saturating_add(cost_nano);
                BillingModelSummary {
                    model,
                    requests,
                    prompt_tokens,
                    cache_read_tokens,
                    completion_tokens,
                    reasoning_tokens,
                    cost_usd: cost_nano as f64 / 1_000_000_000.0,
                }
            },
        )
        .collect();

    (models, total_cost_nano)
}

/// Get aggregate billing summary grouped by model for internal reconciliation.
///
/// start/end are optional YYYY-MM-DD date strings.
/// Pre-migration rows (model IS NULL) are counted separately.
pub async fn get_billing_summary(
    db: &Database,
    start: Option<&str>,
    end: Option<&str>,
) -> Result<BillingSummary> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    let (date_filter, date_cast_start, date_cast_end) =
        billing_date_filter(is_postgres, start, end, 1);

    // Count pre-migration (NULL model) rows
    let null_model_sql = format!(
        "SELECT COUNT(*) FROM router_logs WHERE model IS NULL {}",
        date_filter
    );
    let mut null_query = sqlx::query_scalar::<_, i64>(&null_model_sql);
    if date_cast_start {
        null_query = null_query.bind(start.unwrap_or(""));
    }
    if date_cast_end {
        null_query = null_query.bind(end.unwrap_or(""));
    }
    let pre_migration_requests: i64 = null_query.fetch_one(conn.pool()).await?;

    // Main query: GROUP BY model (exclude NULL model rows from model breakdown)
    let main_sql = format!(
        r#"
        SELECT
            model,
            COUNT(*) as requests,
            COALESCE(SUM(prompt_tokens), 0) as prompt_tokens,
            COALESCE(SUM(cache_read_tokens), 0) as cache_read_tokens,
            COALESCE(SUM(completion_tokens), 0) as completion_tokens,
            COALESCE(SUM(reasoning_tokens), 0) as reasoning_tokens,
            COALESCE(SUM(cost), 0) as cost_nano
        FROM router_logs
        WHERE model IS NOT NULL {}
        GROUP BY model
        ORDER BY cost_nano DESC
        "#,
        date_filter
    );

    let mut main_query = sqlx::query_as::<_, BillingModelRow>(&main_sql);
    if date_cast_start {
        main_query = main_query.bind(start.unwrap_or(""));
    }
    if date_cast_end {
        main_query = main_query.bind(end.unwrap_or(""));
    }
    let rows = main_query.fetch_all(conn.pool()).await?;

    let (models, total_cost_nano) = summarize_billing_rows(rows);

    Ok(BillingSummary {
        period_start: start.map(|s| s.to_string()),
        period_end: end.map(|s| s.to_string()),
        pre_migration_requests,
        total_cost_usd: total_cost_nano as f64 / 1_000_000_000.0,
        models,
    })
}

/// Get per-user billing summary grouped by model.
/// Mirrors `get_billing_summary` but filters by `user_id`.
pub async fn get_billing_summary_for_user(
    db: &Database,
    user_id: &str,
    start: Option<&str>,
    end: Option<&str>,
) -> Result<BillingSummary> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    // `user_id` is bound first, so the date parameters start at placeholder 2.
    let (date_filter, date_cast_start, date_cast_end) =
        billing_date_filter(is_postgres, start, end, 2);

    // Count pre-migration (NULL model) rows for this user
    let null_model_sql = format!(
        "SELECT COUNT(*) FROM router_logs WHERE model IS NULL AND user_id = {} {}",
        ph(is_postgres, 1),
        date_filter
    );
    let mut null_query = sqlx::query_scalar::<_, i64>(&null_model_sql).bind(user_id);
    if date_cast_start {
        null_query = null_query.bind(start.unwrap_or(""));
    }
    if date_cast_end {
        null_query = null_query.bind(end.unwrap_or(""));
    }
    let pre_migration_requests = null_query.fetch_one(conn.pool()).await?;

    // Per-model aggregation for this user
    let model_sql = format!(
        r#"
        SELECT
            model,
            COUNT(*) as requests,
            COALESCE(SUM(prompt_tokens), 0) as prompt_tokens,
            COALESCE(SUM(cache_read_tokens), 0) as cache_read_tokens,
            COALESCE(SUM(completion_tokens), 0) as completion_tokens,
            COALESCE(SUM(reasoning_tokens), 0) as reasoning_tokens,
            COALESCE(SUM(cost), 0) as cost_nano
        FROM router_logs
        WHERE model IS NOT NULL AND user_id = {} {}
        GROUP BY model
        ORDER BY cost_nano DESC
        "#,
        ph(is_postgres, 1),
        date_filter
    );
    let mut model_query = sqlx::query_as::<_, BillingModelRow>(&model_sql).bind(user_id);
    if date_cast_start {
        model_query = model_query.bind(start.unwrap_or(""));
    }
    if date_cast_end {
        model_query = model_query.bind(end.unwrap_or(""));
    }
    let rows = model_query.fetch_all(conn.pool()).await?;

    let (models, total_cost_nano) = summarize_billing_rows(rows);

    Ok(BillingSummary {
        period_start: start.map(|s| s.to_string()),
        period_end: end.map(|s| s.to_string()),
        pre_migration_requests,
        total_cost_usd: total_cost_nano as f64 / 1_000_000_000.0,
        models,
    })
}

/// Storage policy for request logs (controls verbosity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StoragePolicy {
    /// Full request/response recording (dev/debug environments)
    #[default]
    Full,
    /// Metadata only, no body content (production default)
    Summary,
    /// Skip recording entirely (high traffic)
    None,
}

impl StoragePolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Summary => "summary",
            Self::None => "none",
        }
    }

    /// Parses a stored policy name; unknown values fall back to [`StoragePolicy::Summary`].
    ///
    /// This stays an inherent, infallible accessor: `StoragePolicy` is re-exported by
    /// `burncloud-database-router`, so `StoragePolicy::from_str` is public API, and
    /// replacing it with a `FromStr` implementation would change both the return type
    /// and every call site.
    #[allow(
        clippy::should_implement_trait,
        reason = "public API re-exported by burncloud-database-router: StoragePolicy::from_str(s) -> StoragePolicy is kept as-is for compatibility; implementing FromStr would change the return type to Result and the call shape to s.parse()"
    )]
    pub fn from_str(s: &str) -> Self {
        match s {
            "full" => Self::Full,
            "summary" => Self::Summary,
            "none" => Self::None,
            _ => Self::Summary, // Default to summary for unknown values
        }
    }
}

/// Detailed request/response log for debugging.
/// Linked to router_logs via request_id foreign key.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RouterRequestLog {
    pub id: i64,
    pub request_id: String,

    // Request information (sanitized)
    #[sqlx(default)]
    pub request_body: Option<String>, // JSON string (may be truncated)
    #[sqlx(default)]
    pub request_body_truncated: bool,
    #[sqlx(default)]
    pub request_headers: Option<String>, // JSON string (sanitized)

    // Response information
    #[sqlx(default)]
    pub response_body: Option<String>, // JSON string (may be truncated)
    #[sqlx(default)]
    pub response_body_truncated: bool,
    #[sqlx(default)]
    pub response_status: Option<i32>,

    // Streaming response summary
    #[sqlx(default)]
    pub stream_chunk_count: i32,
    #[sqlx(default)]
    pub stream_first_chunk_latency_ms: Option<i64>,
    #[sqlx(default)]
    pub stream_last_chunk_latency_ms: Option<i64>,

    // Routing decision information
    #[sqlx(default)]
    pub candidates: Option<String>, // JSON array string
    #[sqlx(default)]
    pub candidates_count: i32,
    #[sqlx(default)]
    pub affinity_key: Option<String>,
    #[sqlx(default)]
    pub affinity_hit_channel_id: Option<i32>,
    #[sqlx(default)]
    pub failover_history: Option<String>, // JSON array string

    // Storage policy
    #[sqlx(default)]
    pub storage_policy: String,
    #[sqlx(default)]
    pub created_at: Option<String>,
}

/// Failover attempt record for debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverAttempt {
    pub attempt: u32,
    pub channel_id: String,
    pub channel_name: String,
    pub error: Option<String>,
    pub latency_ms: u64,
}

/// Candidate channel information for logging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateInfo {
    pub id: String,
    pub name: String,
    pub protocol: String,
    pub priority: i32,
}

pub struct RouterRequestLogModel;

impl RouterRequestLogModel {
    /// Insert a new request log entry
    pub async fn insert(db: &Database, log: &RouterRequestLog) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = format!(
            r#"
            INSERT INTO router_request_logs
            (request_id, request_body, request_body_truncated, request_headers,
             response_body, response_body_truncated, response_status,
             stream_chunk_count, stream_first_chunk_latency_ms, stream_last_chunk_latency_ms,
             candidates, candidates_count, affinity_key, affinity_hit_channel_id,
             failover_history, storage_policy)
            VALUES ({})
            "#,
            phs(is_postgres, 16)
        );

        sqlx::query(&sql)
            .bind(&log.request_id)
            .bind(&log.request_body)
            .bind(log.request_body_truncated)
            .bind(&log.request_headers)
            .bind(&log.response_body)
            .bind(log.response_body_truncated)
            .bind(log.response_status)
            .bind(log.stream_chunk_count)
            .bind(log.stream_first_chunk_latency_ms)
            .bind(log.stream_last_chunk_latency_ms)
            .bind(&log.candidates)
            .bind(log.candidates_count)
            .bind(&log.affinity_key)
            .bind(log.affinity_hit_channel_id)
            .bind(&log.failover_history)
            .bind(&log.storage_policy)
            .execute(conn.pool())
            .await?;

        Ok(())
    }

    /// Get request log by request_id
    pub async fn get_by_request_id(
        db: &Database,
        request_id: &str,
    ) -> Result<Option<RouterRequestLog>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = format!(
            "SELECT * FROM router_request_logs WHERE request_id = {}",
            ph(is_postgres, 1)
        );

        let log = sqlx::query_as::<_, RouterRequestLog>(&sql)
            .bind(request_id)
            .fetch_optional(conn.pool())
            .await?;

        Ok(log)
    }

    /// Delete request logs older than a threshold (for cleanup)
    pub async fn delete_old_logs(db: &Database, days: i32) -> Result<u64> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = if is_postgres {
            "DELETE FROM router_request_logs WHERE created_at < NOW() - INTERVAL '1 day' * $1"
        } else {
            "DELETE FROM router_request_logs WHERE datetime(created_at) < datetime('now', '-' || ? || ' days')"
        };

        let result = sqlx::query(sql).bind(days).execute(conn.pool()).await?;

        Ok(result.rows_affected())
    }
}

/// Identity owns every write to user account balances. Keep the historical
/// import path until the Traffic crate convergence, without duplicate SQL.
pub use burncloud_service_user::BalanceModel;
