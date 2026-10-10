use crate::rows::AbilityRow;
use burncloud_database::{ph, phs, Database, Result};
use burncloud_supply_contracts::Ability;
use serde::{Deserialize, Serialize};

/// Input for creating a channel ability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelAbilityInput {
    pub group: String,
    pub model: String,
    pub channel_id: i32,
    pub enabled: bool,
    pub priority: i64,
    pub weight: i32,
}

pub struct ChannelAbilityModel;

impl ChannelAbilityModel {
    /// Create abilities in batch for a channel
    ///
    /// This is more efficient than creating abilities one by one.
    /// Handles conflicts by using INSERT OR IGNORE / ON CONFLICT DO NOTHING.
    pub async fn create_batch(db: &Database, abilities: &[ChannelAbilityInput]) -> Result<usize> {
        if abilities.is_empty() {
            return Ok(0);
        }

        let conn = db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = db.kind() == "postgres";
        let group_col = if is_postgres { "\"group\"" } else { "`group`" };

        let mut count = 0;
        for ability in abilities {
            let sql = if is_postgres {
                format!(
                    r#"
                    INSERT INTO channel_abilities ({}, model, channel_id, enabled, priority, weight)
                    VALUES ({})
                    ON CONFLICT ({}, model, channel_id) DO NOTHING
                    "#,
                    group_col,
                    phs(is_postgres, 6),
                    group_col
                )
            } else {
                format!(
                    r#"
                    INSERT OR IGNORE INTO channel_abilities ({}, model, channel_id, enabled, priority, weight)
                    VALUES ({})
                    "#,
                    group_col,
                    phs(is_postgres, 6)
                )
            };

            let result = sqlx::query(&sql)
                .bind(&ability.group)
                .bind(&ability.model)
                .bind(ability.channel_id)
                .bind(ability.enabled)
                .bind(ability.priority)
                .bind(ability.weight)
                .execute(pool)
                .await?;

            count += result.rows_affected() as usize;
        }

        Ok(count)
    }

    /// Delete all abilities for a channel
    pub async fn delete_by_channel(db: &Database, channel_id: i32) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = format!(
            "DELETE FROM channel_abilities WHERE channel_id = {}",
            ph(is_postgres, 1)
        );

        sqlx::query(&sql)
            .bind(channel_id)
            .execute(conn.pool())
            .await?;

        Ok(())
    }

    /// List abilities for a channel
    pub async fn list_by_channel(db: &Database, channel_id: i32) -> Result<Vec<Ability>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let group_col = if is_postgres { "\"group\"" } else { "`group`" };

        let sql = if is_postgres {
            format!(
                "SELECT {} as \"group\", model, channel_id, enabled, priority, weight FROM channel_abilities WHERE channel_id = {}",
                group_col, ph(is_postgres, 1)
            )
        } else {
            format!(
                "SELECT {} as `group`, model, channel_id, enabled, priority, weight FROM channel_abilities WHERE channel_id = {}",
                group_col, ph(is_postgres, 1)
            )
        };

        let abilities = sqlx::query_as::<_, AbilityRow>(&sql)
            .bind(channel_id)
            .fetch_all(conn.pool())
            .await?
            .into_iter()
            .map(Ability::from)
            .collect();

        Ok(abilities)
    }

    /// List enabled abilities at the highest priority for one group/model pair.
    ///
    /// Traffic may rank the returned candidates, but the Supply owner keeps the table/dialect SQL.
    /// The two-step query intentionally preserves the previous routing behavior exactly.
    pub async fn list_enabled_at_highest_priority(
        db: &Database,
        group: &str,
        model: &str,
    ) -> Result<Vec<Ability>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let group_col = if is_postgres { "\"group\"" } else { "`group`" };
        let enabled_lit = if is_postgres { "true" } else { "1" };

        let max_priority_sql = format!(
            "SELECT priority FROM channel_abilities WHERE {group_col} = {} AND model = {} AND enabled = {enabled_lit} ORDER BY priority DESC LIMIT 1",
            ph(is_postgres, 1),
            ph(is_postgres, 2),
        );
        let max_priority: Option<i64> = sqlx::query_scalar(&max_priority_sql)
            .bind(group)
            .bind(model)
            .fetch_optional(conn.pool())
            .await?;

        let Some(priority) = max_priority else {
            return Ok(Vec::new());
        };

        let select_sql = if is_postgres {
            format!(
                "SELECT {group_col} as \"group\", model, channel_id, enabled, priority, weight FROM channel_abilities WHERE {group_col} = {} AND model = {} AND enabled = {enabled_lit} AND priority = {}",
                ph(is_postgres, 1),
                ph(is_postgres, 2),
                ph(is_postgres, 3),
            )
        } else {
            format!(
                "SELECT {group_col} as `group`, model, channel_id, enabled, priority, weight FROM channel_abilities WHERE {group_col} = {} AND model = {} AND enabled = {enabled_lit} AND priority = {}",
                ph(is_postgres, 1),
                ph(is_postgres, 2),
                ph(is_postgres, 3),
            )
        };

        let abilities = sqlx::query_as::<_, AbilityRow>(&select_sql)
            .bind(group)
            .bind(model)
            .bind(priority)
            .fetch_all(conn.pool())
            .await?
            .into_iter()
            .map(Ability::from)
            .collect();
        Ok(abilities)
    }

    /// List all distinct models from channel_abilities
    ///
    /// Returns a list of unique model names that have at least one enabled channel.
    /// Used by /v1/models endpoint to show available models.
    pub async fn list_distinct_models(db: &Database) -> Result<Vec<String>> {
        let conn = db.get_connection()?;

        let sql = "SELECT DISTINCT model FROM channel_abilities WHERE enabled = 1 ORDER BY model";

        let models: Vec<(String,)> = sqlx::query_as(sql).fetch_all(conn.pool()).await?;

        Ok(models.into_iter().map(|(m,)| m).collect())
    }
}
