use crate::common::current_timestamp;
use crate::rows::ChannelRow;
use burncloud_database::{adapt_sql, ph, phs, Database, Result};
use burncloud_supply_contracts::Channel;
use sqlx::Row;

pub struct ChannelProviderModel;

impl ChannelProviderModel {
    pub async fn create(db: &Database, channel: &mut Channel) -> Result<i32> {
        let conn = db.get_connection()?;
        let pool = conn.pool();

        let is_postgres = db.kind() == "postgres";
        let group_col = if is_postgres { "\"group\"" } else { "`group`" };
        let type_col = if is_postgres { "\"type\"" } else { "type" };

        // Basic Insert
        let sql = if is_postgres {
            format!(
                r#"
                INSERT INTO channel_providers ({}, key, status, name, weight, base_url, models, {}, model_mapping, priority, other_info, tag, setting, created_time, param_override, header_override, remark, api_version, pricing_region, rpm_cap, tpm_cap, reservation_green, reservation_yellow, reservation_red)
                VALUES ({})
                RETURNING id
                "#,
                type_col,
                group_col,
                phs(is_postgres, 24)
            )
        } else {
            format!(
                r#"
                INSERT INTO channel_providers ({}, key, status, name, weight, base_url, models, {}, model_mapping, priority, other_info, tag, setting, created_time, param_override, header_override, remark, api_version, pricing_region, rpm_cap, tpm_cap, reservation_green, reservation_yellow, reservation_red)
                VALUES ({})
                "#,
                type_col,
                group_col,
                phs(is_postgres, 24)
            )
        };

        let now = current_timestamp();
        let mut pending = channel.clone();
        pending.created_time = Some(now);

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
            .bind(&channel.model_mapping)
            .bind(channel.priority)
            .bind(&channel.other_info)
            .bind(&channel.tag)
            .bind(&channel.setting)
            .bind(pending.created_time)
            .bind(&channel.param_override)
            .bind(&channel.header_override)
            .bind(&channel.remark)
            .bind(&channel.api_version)
            .bind(&channel.pricing_region)
            .bind(channel.rpm_cap)
            .bind(channel.tpm_cap)
            .bind(channel.reservation_green)
            .bind(channel.reservation_yellow)
            .bind(channel.reservation_red);

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

        // Do not publish ID or timestamp until every database write commits.
        pending.id = id;
        Self::sync_abilities_on(&mut tx, is_postgres, &pending).await?;
        tx.commit().await?;
        channel.id = id;
        channel.created_time = Some(now);

        Ok(id)
    }

    pub async fn update(db: &Database, channel: &Channel) -> Result<()> {
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
            SET {} = ?, key = ?, status = ?, name = ?, weight = ?, base_url = ?, models = ?, {} = ?, model_mapping = ?, priority = ?, other_info = ?, tag = ?, setting = ?, param_override = ?, header_override = ?, remark = ?, api_version = ?, pricing_region = ?, rpm_cap = ?, tpm_cap = ?, reservation_green = ?, reservation_yellow = ?, reservation_red = ?
            WHERE id = ?
            "#,
                type_col, group_col
            ),
        );

        let mut tx = pool.begin().await?;
        sqlx::query(&sql)
            .bind(channel.type_)
            .bind(&channel.key)
            .bind(channel.status)
            .bind(&channel.name)
            .bind(channel.weight)
            .bind(&channel.base_url)
            .bind(&channel.models)
            .bind(&channel.group)
            .bind(&channel.model_mapping)
            .bind(channel.priority)
            .bind(&channel.other_info)
            .bind(&channel.tag)
            .bind(&channel.setting)
            .bind(&channel.param_override)
            .bind(&channel.header_override)
            .bind(&channel.remark)
            .bind(&channel.api_version)
            .bind(&channel.pricing_region)
            .bind(channel.rpm_cap)
            .bind(channel.tpm_cap)
            .bind(channel.reservation_green)
            .bind(channel.reservation_yellow)
            .bind(channel.reservation_red)
            .bind(channel.id)
            .execute(&mut *tx)
            .await?;

        Self::sync_abilities_on(&mut tx, is_postgres, channel).await?;
        tx.commit().await?;
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

    /// Update only the provider API version while keeping all other Channel fields unchanged.
    ///
    /// This is the Supply-owned boundary used by Traffic's deprecation detector. The historical
    /// caller treated a missing id as a successful no-op, so this targeted update intentionally
    /// preserves that behavior instead of turning it into a not-found error.
    pub async fn update_api_version(db: &Database, id: i32, api_version: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = adapt_sql(
            is_postgres,
            "UPDATE channel_providers SET api_version = ? WHERE id = ?",
        );

        sqlx::query(&sql)
            .bind(api_version)
            .bind(id)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    /// Atomically quarantine a Channel: remove its routing abilities and mark it disabled.
    ///
    /// Returns `Ok(false)` when the provider no longer exists. In that case the transaction is
    /// dropped, rolling back the ability deletion so callers never observe a half-quarantined row.
    pub async fn quarantine(db: &Database, id: i32) -> Result<bool> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let mut tx = conn.pool().begin().await?;

        let delete_abilities = adapt_sql(
            is_postgres,
            "DELETE FROM channel_abilities WHERE channel_id = ?",
        );
        sqlx::query(&delete_abilities)
            .bind(id)
            .execute(&mut *tx)
            .await?;

        let disable_channel = adapt_sql(
            is_postgres,
            "UPDATE channel_providers SET status = 3 WHERE id = ?",
        );
        let result = sqlx::query(&disable_channel)
            .bind(id)
            .execute(&mut *tx)
            .await?;

        if result.rows_affected() != 1 {
            return Ok(false);
        }

        tx.commit().await?;
        Ok(true)
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

        let channel = sqlx::query_as::<_, ChannelRow>(&sql)
            .bind(id)
            .fetch_optional(conn.pool())
            .await?
            .map(Channel::from);

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

        let channels = sqlx::query_as::<_, ChannelRow>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(conn.pool())
            .await?
            .into_iter()
            .map(Channel::from)
            .collect();

        Ok(channels)
    }

    /// List channels by their ids.
    ///
    /// Used by the routing layer to resolve a candidate id set into channel details. The SQL
    /// lives here rather than in the caller: the routing layer must not read the channel
    /// table itself (see #607).
    pub async fn list_by_ids(db: &Database, ids: &[i32]) -> Result<Vec<Channel>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let placeholders = phs(is_postgres, ids.len());

        // Identifier quoting differs by dialect (`type as "type_"`, "group" vs `group`).
        let sql = if is_postgres {
            format!(
                r#"
                SELECT
                    id, type as "type_", key, status, name, weight, created_time, test_time,
                    response_time, base_url, models, "group", used_quota, model_mapping,
                    priority, auto_ban, other_info, tag, setting, param_override,
                    header_override, remark, api_version, pricing_region,
                    rpm_cap, tpm_cap, reservation_green, reservation_yellow, reservation_red
                FROM channel_providers WHERE id IN ({})
                "#,
                placeholders
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
                FROM channel_providers WHERE id IN ({})
                "#,
                placeholders
            )
        };

        let mut query = sqlx::query_as::<_, ChannelRow>(&sql);
        for id in ids {
            query = query.bind(id);
        }

        let channels = query
            .fetch_all(conn.pool())
            .await?
            .into_iter()
            .map(Channel::from)
            .collect();

        Ok(channels)
    }

    pub async fn sync_abilities(db: &Database, channel: &Channel) -> Result<()> {
        let conn = db.get_connection()?;
        let mut tx = conn.pool().begin().await?;
        Self::sync_abilities_on(&mut tx, db.kind() == "postgres", channel).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn sync_abilities_on(
        tx: &mut sqlx::Transaction<'_, sqlx::Any>,
        is_postgres: bool,
        channel: &Channel,
    ) -> Result<()> {
        // 1. Delete existing abilities for this channel
        let sql_delete = adapt_sql(
            is_postgres,
            "DELETE FROM channel_abilities WHERE channel_id = ?",
        );
        sqlx::query(&sql_delete)
            .bind(channel.id)
            .execute(&mut **tx)
            .await?;

        // 2. Add new abilities
        if channel.status != 1 {
            // If channel disabled, don't add abilities
            return Ok(());
        }

        let models: Vec<&str> = channel
            .models
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        let groups: Vec<&str> = channel
            .group
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        let group_col = if is_postgres { "\"group\"" } else { "`group`" };

        let sql_insert = adapt_sql(
            is_postgres,
            &format!(
                r#"
            INSERT INTO channel_abilities ({}, model, channel_id, enabled, priority, weight)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
                group_col
            ),
        );

        // Mapping aliases commonly point at a model already present in `models`. Keep the
        // existing visible uniqueness error for duplicate `models`/`group` entries, but make
        // mapping-derived abilities idempotent against that already-declared capability.
        let sql_insert_mapping = adapt_sql(
            is_postgres,
            &format!(
                r#"
            INSERT INTO channel_abilities ({}, model, channel_id, enabled, priority, weight)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT ({}, model, channel_id) DO NOTHING
            "#,
                group_col, group_col
            ),
        );

        for model in models {
            for group in &groups {
                tracing::info!(
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
                    .execute(&mut **tx)
                    .await?;
            }
        }

        // Insert abilities for model_mapping field (both keys and values)
        if let Some(model_mapping_str) = &channel.model_mapping {
            if let Ok(mapping) =
                serde_json::from_str::<std::collections::HashMap<String, String>>(model_mapping_str)
            {
                for (key, value) in &mapping {
                    // Normalize to lowercase
                    let key_lower = key.to_lowercase();
                    let value_lower = value.to_lowercase();
                    for group in &groups {
                        // Insert for key (user-facing model name)
                        tracing::info!(
                            "ChannelProviderModel: Inserting ability from model_mapping key - Key: {}, Group: {}, ChannelID: {}",
                            key_lower,
                            group,
                            channel.id
                        );
                        sqlx::query(&sql_insert_mapping)
                            .bind(group)
                            .bind(&key_lower)
                            .bind(channel.id)
                            .bind(true)
                            .bind(channel.priority)
                            .bind(channel.weight)
                            .execute(&mut **tx)
                            .await?;

                        // Insert for value (actual model name)
                        tracing::info!(
                            "ChannelProviderModel: Inserting ability from model_mapping value - Value: {}, Group: {}, ChannelID: {}",
                            value_lower,
                            group,
                            channel.id
                        );
                        sqlx::query(&sql_insert_mapping)
                            .bind(group)
                            .bind(&value_lower)
                            .bind(channel.id)
                            .bind(true)
                            .bind(channel.priority)
                            .bind(channel.weight)
                            .execute(&mut **tx)
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }
}
