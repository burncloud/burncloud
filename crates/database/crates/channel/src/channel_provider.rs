use crate::common::current_timestamp;
use burncloud_common::types::Channel;
use burncloud_database::{adapt_sql, ph, phs, Database, Result};
use sqlx::Row;

pub struct ChannelProviderModel;

impl ChannelProviderModel {
    pub async fn create(db: &Database, channel: &mut Channel) -> Result<i32> {
        // Ensure model_mapping is never NULL — store "{}" instead so
        // sync_abilities and runtime proxy_logic always have data to read.
        if channel.model_mapping.is_none() {
            channel.model_mapping = Some("{}".to_string());
        }

        // Normalize models to lowercase so channel_abilities lookups (which
        // also lowercase the query) always match regardless of request casing.
        channel.models = channel
            .models
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(",");

        // Normalize group to lowercase for the same reason.
        channel.group = channel
            .group
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(",");

        let conn = db.get_connection()?;
        let pool = conn.pool();

        let is_postgres = db.kind() == "postgres";
        let group_col = if is_postgres { "\"group\"" } else { "`group`" };
        let type_col = if is_postgres { "\"type\"" } else { "type" };

        // Basic Insert
        let sql = if is_postgres {
            format!(
                r#"
                INSERT INTO channel_providers ({}, key, status, name, weight, base_url, models, {}, priority, created_time, param_override, header_override, api_version, pricing_region, rpm_cap, tpm_cap, reservation_green, reservation_yellow, reservation_red, model_mapping)
                VALUES ({})
                RETURNING id
                "#,
                type_col,
                group_col,
                phs(is_postgres, 20)
            )
        } else {
            format!(
                r#"
                INSERT INTO channel_providers ({}, key, status, name, weight, base_url, models, {}, priority, created_time, param_override, header_override, api_version, pricing_region, rpm_cap, tpm_cap, reservation_green, reservation_yellow, reservation_red, model_mapping)
                VALUES ({})
                "#,
                type_col,
                group_col,
                phs(is_postgres, 20)
            )
        };

        let now = current_timestamp();
        channel.created_time = Some(now);

        let model_mapping_normalized = channel
            .model_mapping
            .as_deref()
            .unwrap_or("{}");

        // Use transaction to ensure last_insert_rowid works on the same connection
        let mut tx = pool.begin().await?;

        let query = sqlx::query(&sql)
            .bind(channel.type_)
            .bind(&channel.key)
            .bind(channel.status)
            .bind(&channel.name)
            .bind(channel.weight)
            .bind(&channel.base_url)
            .bind(&channel.models)
            .bind(&channel.group)
            .bind(channel.priority)
            .bind(channel.created_time)
            .bind(&channel.param_override)
            .bind(&channel.header_override)
            .bind(&channel.api_version)
            .bind(&channel.pricing_region)
            .bind(channel.rpm_cap)
            .bind(channel.tpm_cap)
            .bind(channel.reservation_green)
            .bind(channel.reservation_yellow)
            .bind(channel.reservation_red)
            .bind(model_mapping_normalized);

        let id = if db.kind() == "postgres" {
            let row = query.fetch_one(&mut *tx).await?;
            row.get::<i32, _>(0)
        } else {
            query.execute(&mut *tx).await?;
            // For SQLite with AnyPool, we need a separate query to get ID on the SAME connection (transaction)
            let row: (i64,) = sqlx::query_as("SELECT last_insert_rowid()")
                .fetch_one(&mut *tx)
                .await?;
            row.0 as i32
        };

        tx.commit().await?;

        channel.id = id;

        Self::sync_abilities(db, channel).await?;

        Ok(id)
    }

    pub async fn update(db: &Database, channel: &Channel) -> Result<()> {
        // Ensure model_mapping is never NULL — store "{}" instead so
        // sync_abilities and runtime proxy_logic always have data to read.
        let model_mapping_normalized = channel
            .model_mapping
            .as_deref()
            .unwrap_or("{}");

        // Normalize models to lowercase — same logic as create() so
        // channel_providers.models is always consistent regardless of
        // whether the channel was created or updated.
        let models_normalized = channel
            .models
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(",");

        // Normalize group to lowercase — same reason.
        let group_normalized = channel
            .group
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(",");

        let conn = db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = db.kind() == "postgres";

        let group_col = if is_postgres { "\"group\"" } else { "`group`" };
        let type_col = if is_postgres { "\"type\"" } else { "type" };

        let sql = adapt_sql(
            is_postgres,
            &format!(
                r#"
            UPDATE channel_providers
            SET {} = ?, key = ?, status = ?, name = ?, weight = ?, base_url = ?, models = ?, {} = ?, priority = ?, param_override = ?, header_override = ?, api_version = ?, pricing_region = ?, rpm_cap = ?, tpm_cap = ?, reservation_green = ?, reservation_yellow = ?, reservation_red = ?, model_mapping = ?
            WHERE id = ?
            "#,
                type_col, group_col
            ),
        );

        sqlx::query(&sql)
            .bind(channel.type_)
            .bind(&channel.key)
            .bind(channel.status)
            .bind(&channel.name)
            .bind(channel.weight)
            .bind(&channel.base_url)
            .bind(&models_normalized)
            .bind(&group_normalized)
            .bind(channel.priority)
            .bind(&channel.param_override)
            .bind(&channel.header_override)
            .bind(&channel.api_version)
            .bind(&channel.pricing_region)
            .bind(channel.rpm_cap)
            .bind(channel.tpm_cap)
            .bind(channel.reservation_green)
            .bind(channel.reservation_yellow)
            .bind(channel.reservation_red)
            .bind(model_mapping_normalized)
            .bind(channel.id)
            .execute(pool)
            .await?;

        // Sync abilities using normalized values (not original channel object)
        // to ensure channel_abilities matches what's stored in the database.
        let mut normalized_channel = channel.clone();
        normalized_channel.models = models_normalized;
        normalized_channel.group = group_normalized;
        normalized_channel.model_mapping = Some(model_mapping_normalized.to_string());
        Self::sync_abilities(db, &normalized_channel).await?;
        Ok(())
    }

    pub async fn delete(db: &Database, id: i32) -> Result<()> {
        let conn = db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = db.kind() == "postgres";

        // Delete Abilities first
        let sql_abilities = adapt_sql(
            is_postgres,
            "DELETE FROM channel_abilities WHERE channel_id = ?",
        );
        sqlx::query(&sql_abilities).bind(id).execute(pool).await?;

        // Delete Channel
        let sql_channels = adapt_sql(is_postgres, "DELETE FROM channel_providers WHERE id = ?");
        sqlx::query(&sql_channels).bind(id).execute(pool).await?;

        Ok(())
    }

    pub async fn get_by_id(db: &Database, id: i32) -> Result<Option<Channel>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = if is_postgres {
            format!(
                r#"
                SELECT
                    id, type as "type_", key, status, name, weight, created_time, test_time,
                    response_time, base_url, models, "group", used_quota, model_mapping,
                    priority, auto_ban, other_info, tag, setting, param_override,
                    header_override, remark, api_version, pricing_region,
                    rpm_cap, tpm_cap, reservation_green, reservation_yellow, reservation_red
                FROM channel_providers WHERE id = {}
            "#,
                ph(is_postgres, 1)
            )
        } else {
            format!(
                r#"
                SELECT
                    id, type as type_, key, status, name, weight, created_time, test_time,
                    response_time, base_url, models, `group`, used_quota, model_mapping,
                    priority, auto_ban, other_info, tag, setting, param_override,
                    header_override, remark, api_version, pricing_region,
                    rpm_cap, tpm_cap, reservation_green, reservation_yellow, reservation_red
                FROM channel_providers WHERE id = {}
            "#,
                ph(is_postgres, 1)
            )
        };

        let channel = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(conn.pool())
            .await?;

        Ok(channel)
    }

    pub async fn list(db: &Database, limit: i32, offset: i32) -> Result<Vec<Channel>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = if is_postgres {
            format!(
                r#"
                SELECT
                    id, type as "type_", key, status, name, weight, created_time, test_time,
                    response_time, base_url, models, "group", used_quota, model_mapping,
                    priority, auto_ban, other_info, tag, setting, param_override,
                    header_override, remark, api_version, pricing_region,
                    rpm_cap, tpm_cap, reservation_green, reservation_yellow, reservation_red
                FROM channel_providers ORDER BY id DESC LIMIT {} OFFSET {}
            "#,
                ph(is_postgres, 1),
                ph(is_postgres, 2)
            )
        } else {
            format!(
                r#"
                SELECT
                    id, type as type_, key, status, name, weight, created_time, test_time,
                    response_time, base_url, models, `group`, used_quota, model_mapping,
                    priority, auto_ban, other_info, tag, setting, param_override,
                    header_override, remark, api_version, pricing_region,
                    rpm_cap, tpm_cap, reservation_green, reservation_yellow, reservation_red
                FROM channel_providers ORDER BY id DESC LIMIT {} OFFSET {}
            "#,
                ph(is_postgres, 1),
                ph(is_postgres, 2)
            )
        };

        let channels = sqlx::query_as(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(conn.pool())
            .await?;

        Ok(channels)
    }

    pub async fn sync_abilities(db: &Database, channel: &Channel) -> Result<()> {
        let conn = db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = db.kind() == "postgres";

        // Wrap DELETE + INSERT in a transaction so concurrent sync_abilities calls
        // cannot observe a window where abilities are missing (which would cause
        // UNIQUE constraint violations or routing misses).
        let mut tx = pool.begin().await?;

        // 1. Delete existing abilities for this channel
        let sql_delete = adapt_sql(
            is_postgres,
            "DELETE FROM channel_abilities WHERE channel_id = ?",
        );
        sqlx::query(&sql_delete)
            .bind(channel.id)
            .execute(&mut *tx)
            .await?;

        // 2. Add new abilities
        if channel.status != 1 {
            // If channel disabled, don't add abilities
            tx.commit().await?;
            return Ok(());
        }

        let models: Vec<String> = channel
            .models
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        // Collect model_mapping keys as additional routable model names
        let mapping_keys: Vec<String> = channel
            .model_mapping
            .as_deref()
            .map(extract_json_keys)
            .unwrap_or_default();

        // Collect model_mapping values so that requesting the upstream model name
        // directly also routes to this channel.
        // Note: values are lowercased here for channel_abilities routing lookup
        // (get_candidates also lowercases its query). The runtime model_mapping
        // substitution in proxy_logic uses the original-case value from the JSON
        // object to preserve the exact model name expected by the upstream API.
        // These two paths serve different purposes and are intentionally asymmetric.
        let mapping_values: Vec<String> = channel
            .model_mapping
            .as_deref()
            .map(extract_json_values)
            .unwrap_or_default();

        let mut all_models: Vec<String> = models
            .into_iter()
            .chain(mapping_keys)
            .chain(mapping_values)
            .collect();
        all_models.sort();
        all_models.dedup();
        let groups: Vec<String> = channel
            .group
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        let group_col = if is_postgres { "\"group\"" } else { "`group`" };

        let sql_insert = if is_postgres {
            adapt_sql(
                is_postgres,
                &format!(
                    r#"
                INSERT INTO channel_abilities ({}, model, channel_id, enabled, priority, weight)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT ({}, model, channel_id) DO UPDATE SET
                    enabled = EXCLUDED.enabled,
                    priority = EXCLUDED.priority,
                    weight = EXCLUDED.weight
                "#,
                    group_col, group_col
                ),
            )
        } else {
            format!(
                r#"
            INSERT OR REPLACE INTO channel_abilities ({}, model, channel_id, enabled, priority, weight)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
                group_col
            )
        };

        for model in &all_models {
            for group in &groups {
                tracing::debug!(
                    "ChannelProviderModel: Inserting ability - Model: {}, Group: {}, ChannelID: {}",
                    model,
                    group,
                    channel.id
                );
                sqlx::query(&sql_insert)
                    .bind(group)
                    .bind(model)
                    .bind(channel.id)
                    .bind(true) // sqlx handles boolean mapping
                    .bind(channel.priority)
                    .bind(channel.weight)
                    .execute(&mut *tx)
                    .await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }
}

/// Extract top-level string keys from a JSON object string, returning lowercase keys.
/// Returns an empty list for empty objects, non-objects, or malformed JSON.
#[allow(clippy::disallowed_types)]
fn extract_json_keys(json: &str) -> Vec<String> {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .map(|obj| {
            obj.keys()
                .map(|k| k.trim().to_lowercase())
                .filter(|k| !k.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Extract top-level string values from a JSON object string, returning lowercase values.
/// Returns an empty list for empty objects, non-objects, or malformed JSON.
#[allow(clippy::disallowed_types)]
fn extract_json_values(json: &str) -> Vec<String> {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .map(|obj| {
            obj.values()
                .filter_map(|v| v.as_str())
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_keys_basic() {
        let keys = extract_json_keys(r#"{"gpt-4o-mini": "gpt-4o"}"#);
        assert_eq!(keys, vec!["gpt-4o-mini"]);
    }

    #[test]
    fn test_extract_json_keys_multiple() {
        let keys = extract_json_keys(r#"{"gpt-4o-mini": "gpt-4o", "gpt-4": "gpt-4o"}"#);
        let mut sorted = keys;
        sorted.sort();
        assert_eq!(sorted, vec!["gpt-4", "gpt-4o-mini"]);
    }

    #[test]
    fn test_extract_json_keys_empty() {
        assert!(extract_json_keys("{}").is_empty());
        assert!(extract_json_keys("").is_empty());
        assert!(extract_json_keys("not json").is_empty());
    }

    #[test]
    fn test_extract_json_keys_lowercase() {
        let keys = extract_json_keys(r#"{"GPT-4O": "gpt-4o"}"#);
        assert_eq!(keys, vec!["gpt-4o"]);
    }

    #[test]
    fn test_extract_json_keys_astron_mapping() {
        let keys = extract_json_keys(r#"{"astron-code": "astron-code-latest"}"#);
        assert_eq!(keys, vec!["astron-code"]);
    }

    #[test]
    fn test_extract_json_values_basic() {
        let values = extract_json_values(r#"{"gpt-4o-mini": "gpt-4o"}"#);
        assert_eq!(values, vec!["gpt-4o"]);
    }

    #[test]
    fn test_extract_json_values_multiple() {
        let values = extract_json_values(r#"{"gpt-4o-mini": "gpt-4o", "gpt-4": "gpt-4o"}"#);
        assert_eq!(values, vec!["gpt-4o", "gpt-4o"]);
    }

    #[test]
    fn test_extract_json_values_empty() {
        assert!(extract_json_values("{}").is_empty());
        assert!(extract_json_values("").is_empty());
        assert!(extract_json_values("not json").is_empty());
    }

    #[test]
    fn test_extract_json_values_lowercase() {
        let values = extract_json_values(r#"{"key": "GPT-4O"}"#);
        assert_eq!(values, vec!["gpt-4o"]);
    }

    #[test]
    fn test_extract_json_values_astron_mapping() {
        let values = extract_json_values(r#"{"astron-code": "astron-code-latest"}"#);
        assert_eq!(values, vec!["astron-code-latest"]);
    }
}
