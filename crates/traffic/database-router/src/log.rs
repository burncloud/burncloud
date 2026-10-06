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
    #[sqlx(default)]
    pub layer_decision: Option<String>,
    #[sqlx(default)]
    pub traffic_color: Option<String>,
    #[sqlx(default)]
    pub cost_status: Option<String>,
    #[sqlx(default)]
    pub error_type: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStats {
    pub total_requests: i64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    pub total_cost_nano: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageStats {
    pub model: String,
    pub requests: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cache_read_tokens: i64,
    pub reasoning_tokens: i64,
    pub cost_nano: i64,
}

type UsageAggregateRow = (Option<i64>, Option<i64>, Option<i64>, Option<i64>);
type ModelUsageRow = (String, i64, i64, i64, i64, i64, i64);
type BillingModelRow = (String, i64, i64, i64, i64, i64, i64);

pub struct RouterLogModel;

impl RouterLogModel {
    pub async fn insert(db: &Database, log: &RouterLog) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = format!(
            r#"
            INSERT INTO router_logs
            (request_id, user_id, path, upstream_id, status_code, latency_ms,
             prompt_tokens, completion_tokens, cost,
             model, cache_read_tokens, reasoning_tokens, pricing_region, video_tokens,
             cache_write_tokens, audio_input_tokens, audio_output_tokens, image_tokens, embedding_tokens,
             input_cost, output_cost, cache_read_cost, cache_write_cost,
             audio_cost, image_cost, video_cost, reasoning_cost, embedding_cost,
             layer_decision, traffic_color, cost_status, error_type)
            VALUES ({})
            "#,
            phs(is_postgres, 32)
        );

        sqlx::query(&sql)
            .bind(&log.request_id)
            .bind(&log.user_id)
            .bind(&log.path)
            .bind(&log.upstream_id)
            .bind(log.status_code)
            .bind(log.latency_ms)
            .bind(log.prompt_tokens)
            .bind(log.completion_tokens)
            .bind(log.cost)
            .bind(&log.model)
            .bind(log.cache_read_tokens)
            .bind(log.reasoning_tokens)
            .bind(&log.pricing_region)
            .bind(log.video_tokens)
            .bind(log.cache_write_tokens)
            .bind(log.audio_input_tokens)
            .bind(log.audio_output_tokens)
            .bind(log.image_tokens)
            .bind(log.embedding_tokens)
            .bind(log.input_cost)
            .bind(log.output_cost)
            .bind(log.cache_read_cost)
            .bind(log.cache_write_cost)
            .bind(log.audio_cost)
            .bind(log.image_cost)
            .bind(log.video_cost)
            .bind(log.reasoning_cost)
            .bind(log.embedding_cost)
            .bind(&log.layer_decision)
            .bind(&log.traffic_color)
            .bind(&log.cost_status)
            .bind(&log.error_type)
            .execute(conn.pool())
            .await?;

        if let Some(user_id) = &log.user_id {
            let total_tokens = log.prompt_tokens + log.completion_tokens;
            if total_tokens > 0 {
                let update_sql = adapt_sql(
                    is_postgres,
                    "UPDATE router_tokens SET used_quota = used_quota + ? WHERE user_id = ?",
                );
                sqlx::query(&update_sql)
                    .bind(total_tokens)
                    .bind(user_id)
                    .execute(conn.pool())
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get(db: &Database, limit: i32, offset: i32) -> Result<Vec<RouterLog>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = format!(
            "SELECT id, request_id, user_id, path, upstream_id, status_code, latency_ms, prompt_tokens, completion_tokens, cost, model, cache_read_tokens, reasoning_tokens, pricing_region, video_tokens, cache_write_tokens, audio_input_tokens, audio_output_tokens, image_tokens, embedding_tokens, input_cost, output_cost, cache_read_cost, cache_write_cost, audio_cost, image_cost, video_cost, reasoning_cost, embedding_cost, layer_decision, traffic_color, cost_status, error_type, created_at FROM router_logs ORDER BY created_at DESC {}",
            adapt_sql(is_postgres, "LIMIT ? OFFSET ?")
        );
        Ok(sqlx::query_as::<_, RouterLog>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(conn.pool())
            .await?)
    }

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
        let mut conditions = Vec::new();
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
        if let Some(model) = model {
            query = query.bind(model);
        }
        Ok(query
            .bind(limit)
            .bind(offset)
            .fetch_all(conn.pool())
            .await?)
    }

    pub async fn get_usage_by_user(db: &Database, user_id: &str) -> Result<(i64, i64)> {
        let conn = db.get_connection()?;
        let sql = adapt_sql(
            db.kind() == "postgres",
            "SELECT SUM(prompt_tokens), SUM(completion_tokens) FROM router_logs WHERE user_id = ?",
        );
        let row: (Option<i64>, Option<i64>) = sqlx::query_as(&sql)
            .bind(user_id)
            .fetch_one(conn.pool())
            .await?;
        Ok((row.0.unwrap_or(0), row.1.unwrap_or(0)))
    }
}

fn period_threshold(period: &str) -> Result<i64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| DatabaseError::Query(format!("Time error: {error}")))?
        .as_secs() as i64;
    Ok(match period {
        "day" => now - 24 * 60 * 60,
        "week" => now - 7 * 24 * 60 * 60,
        _ => now - 30 * 24 * 60 * 60,
    })
}

pub async fn get_usage_stats(db: &Database, user_id: &str, period: &str) -> Result<UsageStats> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";
    let threshold = period_threshold(period)?;
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
        SELECT COUNT(*), COALESCE(SUM(prompt_tokens), 0),
               COALESCE(SUM(completion_tokens), 0), COALESCE(SUM(cost), 0)
        FROM router_logs
        WHERE user_id = {} AND created_at IS NOT NULL AND {}
        "#,
        ph(is_postgres, 1),
        time_filter
    );
    let row: UsageAggregateRow = sqlx::query_as(&sql)
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

pub async fn get_usage_stats_by_model(
    db: &Database,
    user_id: &str,
    period: &str,
) -> Result<Vec<ModelUsageStats>> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";
    let threshold = period_threshold(period)?;
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
        SELECT COALESCE(model, 'Unknown'), COUNT(*),
               COALESCE(SUM(prompt_tokens), 0), COALESCE(SUM(completion_tokens), 0),
               COALESCE(SUM(cache_read_tokens), 0), COALESCE(SUM(reasoning_tokens), 0),
               COALESCE(SUM(cost), 0)
        FROM router_logs
        WHERE user_id = {} AND created_at IS NOT NULL AND {}
        GROUP BY model ORDER BY cost DESC
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
        .map(|row| ModelUsageStats {
            model: row.0,
            requests: row.1,
            prompt_tokens: row.2,
            completion_tokens: row.3,
            cache_read_tokens: row.4,
            reasoning_tokens: row.5,
            cost_nano: row.6,
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingModelSummary {
    pub model: String,
    pub requests: i64,
    pub prompt_tokens: i64,
    pub cache_read_tokens: i64,
    pub completion_tokens: i64,
    pub reasoning_tokens: i64,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingSummary {
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub pre_migration_requests: i64,
    pub models: Vec<BillingModelSummary>,
    pub total_cost_usd: f64,
}

struct BillingDateFilter {
    clause: String,
    bind_start: bool,
    bind_end: bool,
}

fn billing_date_filter(
    is_postgres: bool,
    start: Option<&str>,
    end: Option<&str>,
    first_param: usize,
) -> BillingDateFilter {
    let start_ph = ph(is_postgres, first_param);
    let end_ph = ph(is_postgres, first_param + 1);
    let (clause, bind_start, bind_end) = match (start.is_some(), end.is_some(), is_postgres) {
        (true, true, true) => (
            format!("AND created_at::date >= {start_ph}::date AND created_at::date <= {end_ph}::date"),
            true,
            true,
        ),
        (true, false, true) => (format!("AND created_at::date >= {start_ph}::date"), true, false),
        (false, true, true) => (format!("AND created_at::date <= {start_ph}::date"), false, true),
        (true, true, false) => (
            format!("AND strftime('%Y-%m-%d', created_at) >= {start_ph} AND strftime('%Y-%m-%d', created_at) <= {end_ph}"),
            true,
            true,
        ),
        (true, false, false) => (
            format!("AND strftime('%Y-%m-%d', created_at) >= {start_ph}"),
            true,
            false,
        ),
        (false, true, false) => (
            format!("AND strftime('%Y-%m-%d', created_at) <= {start_ph}"),
            false,
            true,
        ),
        (false, false, _) => (String::new(), false, false),
    };
    BillingDateFilter {
        clause,
        bind_start,
        bind_end,
    }
}

fn billing_models(rows: Vec<BillingModelRow>) -> (Vec<BillingModelSummary>, i64) {
    let mut total_cost_nano = 0_i64;
    let models = rows
        .into_iter()
        .map(|row| {
            total_cost_nano = total_cost_nano.saturating_add(row.6);
            BillingModelSummary {
                model: row.0,
                requests: row.1,
                prompt_tokens: row.2,
                cache_read_tokens: row.3,
                completion_tokens: row.4,
                reasoning_tokens: row.5,
                cost_usd: row.6 as f64 / 1_000_000_000.0,
            }
        })
        .collect();
    (models, total_cost_nano)
}

pub async fn get_billing_summary(
    db: &Database,
    start: Option<&str>,
    end: Option<&str>,
) -> Result<BillingSummary> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";
    let filter = billing_date_filter(is_postgres, start, end, 1);

    let null_model_sql = format!(
        "SELECT COUNT(*) FROM router_logs WHERE model IS NULL {}",
        filter.clause
    );
    let mut null_query = sqlx::query_scalar::<_, i64>(&null_model_sql);
    if filter.bind_start {
        null_query = null_query.bind(start.unwrap_or(""));
    }
    if filter.bind_end {
        null_query = null_query.bind(end.unwrap_or(""));
    }
    let pre_migration_requests = null_query.fetch_one(conn.pool()).await?;

    let main_sql = format!(
        r#"
        SELECT model, COUNT(*), COALESCE(SUM(prompt_tokens), 0),
               COALESCE(SUM(cache_read_tokens), 0), COALESCE(SUM(completion_tokens), 0),
               COALESCE(SUM(reasoning_tokens), 0), COALESCE(SUM(cost), 0)
        FROM router_logs WHERE model IS NOT NULL {}
        GROUP BY model ORDER BY cost DESC
        "#,
        filter.clause
    );
    let mut query = sqlx::query_as::<_, BillingModelRow>(&main_sql);
    if filter.bind_start {
        query = query.bind(start.unwrap_or(""));
    }
    if filter.bind_end {
        query = query.bind(end.unwrap_or(""));
    }
    let (models, total_cost_nano) = billing_models(query.fetch_all(conn.pool()).await?);

    Ok(BillingSummary {
        period_start: start.map(str::to_string),
        period_end: end.map(str::to_string),
        pre_migration_requests,
        models,
        total_cost_usd: total_cost_nano as f64 / 1_000_000_000.0,
    })
}

pub async fn get_billing_summary_for_user(
    db: &Database,
    user_id: &str,
    start: Option<&str>,
    end: Option<&str>,
) -> Result<BillingSummary> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";
    let filter = billing_date_filter(is_postgres, start, end, 2);

    let null_model_sql = format!(
        "SELECT COUNT(*) FROM router_logs WHERE model IS NULL AND user_id = {} {}",
        ph(is_postgres, 1),
        filter.clause
    );
    let mut null_query = sqlx::query_scalar::<_, i64>(&null_model_sql).bind(user_id);
    if filter.bind_start {
        null_query = null_query.bind(start.unwrap_or(""));
    }
    if filter.bind_end {
        null_query = null_query.bind(end.unwrap_or(""));
    }
    let pre_migration_requests = null_query.fetch_one(conn.pool()).await?;

    let model_sql = format!(
        r#"
        SELECT model, COUNT(*), COALESCE(SUM(prompt_tokens), 0),
               COALESCE(SUM(cache_read_tokens), 0), COALESCE(SUM(completion_tokens), 0),
               COALESCE(SUM(reasoning_tokens), 0), COALESCE(SUM(cost), 0)
        FROM router_logs WHERE model IS NOT NULL AND user_id = {} {}
        GROUP BY model ORDER BY cost DESC
        "#,
        ph(is_postgres, 1),
        filter.clause
    );
    let mut query = sqlx::query_as::<_, BillingModelRow>(&model_sql).bind(user_id);
    if filter.bind_start {
        query = query.bind(start.unwrap_or(""));
    }
    if filter.bind_end {
        query = query.bind(end.unwrap_or(""));
    }
    let (models, total_cost_nano) = billing_models(query.fetch_all(conn.pool()).await?);

    Ok(BillingSummary {
        period_start: start.map(str::to_string),
        period_end: end.map(str::to_string),
        pre_migration_requests,
        models,
        total_cost_usd: total_cost_nano as f64 / 1_000_000_000.0,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StoragePolicy {
    #[default]
    Full,
    Summary,
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
}

impl std::str::FromStr for StoragePolicy {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match value {
            "full" => Self::Full,
            "summary" => Self::Summary,
            "none" => Self::None,
            _ => Self::Summary,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RouterRequestLog {
    pub id: i64,
    pub request_id: String,
    #[sqlx(default)]
    pub request_body: Option<String>,
    #[sqlx(default)]
    pub request_body_truncated: bool,
    #[sqlx(default)]
    pub request_headers: Option<String>,
    #[sqlx(default)]
    pub response_body: Option<String>,
    #[sqlx(default)]
    pub response_body_truncated: bool,
    #[sqlx(default)]
    pub response_status: Option<i32>,
    #[sqlx(default)]
    pub stream_chunk_count: i32,
    #[sqlx(default)]
    pub stream_first_chunk_latency_ms: Option<i64>,
    #[sqlx(default)]
    pub stream_last_chunk_latency_ms: Option<i64>,
    #[sqlx(default)]
    pub candidates: Option<String>,
    #[sqlx(default)]
    pub candidates_count: i32,
    #[sqlx(default)]
    pub affinity_key: Option<String>,
    #[sqlx(default)]
    pub affinity_hit_channel_id: Option<i32>,
    #[sqlx(default)]
    pub failover_history: Option<String>,
    #[sqlx(default)]
    pub storage_policy: String,
    #[sqlx(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverAttempt {
    pub attempt: u32,
    pub channel_id: String,
    pub channel_name: String,
    pub error: Option<String>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateInfo {
    pub id: String,
    pub name: String,
    pub protocol: String,
    pub priority: i32,
}

pub struct RouterRequestLogModel;

impl RouterRequestLogModel {
    pub async fn insert(db: &Database, log: &RouterRequestLog) -> Result<()> {
        let conn = db.get_connection()?;
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
            phs(db.kind() == "postgres", 16)
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

    pub async fn get_by_request_id(
        db: &Database,
        request_id: &str,
    ) -> Result<Option<RouterRequestLog>> {
        let conn = db.get_connection()?;
        let sql = format!(
            "SELECT * FROM router_request_logs WHERE request_id = {}",
            ph(db.kind() == "postgres", 1)
        );
        Ok(sqlx::query_as::<_, RouterRequestLog>(&sql)
            .bind(request_id)
            .fetch_optional(conn.pool())
            .await?)
    }

    pub async fn delete_old_logs(db: &Database, days: i32) -> Result<u64> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "DELETE FROM router_request_logs WHERE created_at < NOW() - INTERVAL '1 day' * $1"
        } else {
            "DELETE FROM router_request_logs WHERE datetime(created_at) < datetime('now', '-' || ? || ' days')"
        };
        Ok(sqlx::query(sql)
            .bind(days)
            .execute(conn.pool())
            .await?
            .rows_affected())
    }
}

pub struct BalanceModel;

impl BalanceModel {
    pub async fn deduct_usd(db: &Database, user_id: &str, cost_nano: i64) -> Result<bool> {
        if cost_nano <= 0 {
            return Ok(true);
        }
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let balance_sql = adapt_sql(
            is_postgres,
            "SELECT COALESCE(balance_usd, 0) FROM user_accounts WHERE id = ?",
        );
        let balance: i64 = sqlx::query_scalar(&balance_sql)
            .bind(user_id)
            .fetch_optional(conn.pool())
            .await?
            .ok_or_else(|| DatabaseError::Query(format!("user account not found: {user_id}")))?;
        if balance < cost_nano {
            return Ok(false);
        }
        let deduct_sql = adapt_sql(
            is_postgres,
            "UPDATE user_accounts SET balance_usd = balance_usd - ? WHERE id = ? AND balance_usd >= ?",
        );
        Ok(sqlx::query(&deduct_sql)
            .bind(cost_nano)
            .bind(user_id)
            .bind(cost_nano)
            .execute(conn.pool())
            .await?
            .rows_affected()
            > 0)
    }

    pub async fn deduct_cny(db: &Database, user_id: &str, cost_nano: i64) -> Result<bool> {
        if cost_nano <= 0 {
            return Ok(true);
        }
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let balance_sql = adapt_sql(
            is_postgres,
            "SELECT COALESCE(balance_cny, 0) FROM user_accounts WHERE id = ?",
        );
        let balance: i64 = sqlx::query_scalar(&balance_sql)
            .bind(user_id)
            .fetch_optional(conn.pool())
            .await?
            .ok_or_else(|| DatabaseError::Query(format!("user account not found: {user_id}")))?;
        if balance < cost_nano {
            return Ok(false);
        }
        let deduct_sql = adapt_sql(
            is_postgres,
            "UPDATE user_accounts SET balance_cny = balance_cny - ? WHERE id = ? AND balance_cny >= ?",
        );
        Ok(sqlx::query(&deduct_sql)
            .bind(cost_nano)
            .bind(user_id)
            .bind(cost_nano)
            .execute(conn.pool())
            .await?
            .rows_affected()
            > 0)
    }

    pub async fn deduct_dual_currency(
        db: &Database,
        user_id: &str,
        cost_nano: i64,
        cost_currency: &str,
        exchange_rate_nano: i64,
    ) -> Result<bool> {
        if cost_nano <= 0 {
            return Ok(true);
        }
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let balances_sql = adapt_sql(
            is_postgres,
            "SELECT COALESCE(balance_usd, 0), COALESCE(balance_cny, 0) FROM user_accounts WHERE id = ?",
        );
        let balances: Option<(i64, i64)> = sqlx::query_as(&balances_sql)
            .bind(user_id)
            .fetch_optional(conn.pool())
            .await?;
        let (balance_usd, balance_cny) = balances
            .ok_or_else(|| DatabaseError::Query(format!("user account not found: {user_id}")))?;

        if cost_currency == "CNY" {
            if balance_cny >= cost_nano {
                return Self::deduct_cny(db, user_id, cost_nano).await;
            }
            let required_cny = cost_nano - balance_cny;
            let required_usd =
                (required_cny as i128 * 1_000_000_000) / exchange_rate_nano as i128;
            if required_usd > balance_usd as i128 {
                return Ok(false);
            }
            let mut tx = conn.pool().begin().await?;
            if balance_cny > 0 {
                let sql = adapt_sql(
                    is_postgres,
                    "UPDATE user_accounts SET balance_cny = 0 WHERE id = ?",
                );
                sqlx::query(&sql).bind(user_id).execute(&mut *tx).await?;
            }
            let amount = required_usd as i64;
            let sql = adapt_sql(
                is_postgres,
                "UPDATE user_accounts SET balance_usd = balance_usd - ? WHERE id = ? AND balance_usd >= ?",
            );
            sqlx::query(&sql)
                .bind(amount)
                .bind(user_id)
                .bind(amount)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok(true);
        }

        if balance_usd >= cost_nano {
            return Self::deduct_usd(db, user_id, cost_nano).await;
        }
        let required_usd = cost_nano - balance_usd;
        let required_cny =
            (required_usd as i128 * exchange_rate_nano as i128) / 1_000_000_000;
        if required_cny > balance_cny as i128 {
            return Ok(false);
        }
        let mut tx = conn.pool().begin().await?;
        if balance_usd > 0 {
            let sql = adapt_sql(
                is_postgres,
                "UPDATE user_accounts SET balance_usd = 0 WHERE id = ?",
            );
            sqlx::query(&sql).bind(user_id).execute(&mut *tx).await?;
        }
        let amount = required_cny as i64;
        let sql = adapt_sql(
            is_postgres,
            "UPDATE user_accounts SET balance_cny = balance_cny - ? WHERE id = ? AND balance_cny >= ?",
        );
        sqlx::query(&sql)
            .bind(amount)
            .bind(user_id)
            .bind(amount)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }
}
