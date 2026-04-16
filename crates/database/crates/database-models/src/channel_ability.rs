use burncloud_common::types::Ability;
use burncloud_database::{ph, phs, Database, Result};
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
                    group_col, phs(is_postgres, 6), group_col
                )
            } else {
                format!(
                    r#"
                    INSERT OR IGNORE INTO channel_abilities ({}, model, channel_id, enabled, priority, weight)
                    VALUES ({})
                    "#,
                    group_col, phs(is_postgres, 6)
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

        let abilities = sqlx::query_as(&sql)
            .bind(channel_id)
            .fetch_all(conn.pool())
            .await?;

        Ok(abilities)
    }
}
