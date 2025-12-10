use anyhow::Result;
use burncloud_common::types::Channel;
use burncloud_database::sqlx;
use burncloud_database::Database;
use rand::Rng; // Use re-exported sqlx

pub struct ModelRouter {
    db: std::sync::Arc<Database>,
}

impl ModelRouter {
    pub fn new(db: std::sync::Arc<Database>) -> Self {
        Self { db }
    }

    /// Routes a request to a suitable channel based on user group and requested model.
    pub async fn route(&self, group: &str, model: &str) -> Result<Option<Channel>> {
        let conn = self.db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = self.db.kind() == "postgres";

        let group_col = if is_postgres { "\"group\"" } else { "`group`" };

        // 1. Get max priority
        let query = if is_postgres {
            format!(
                r#"
                SELECT priority 
                FROM abilities 
                WHERE {} = $1 AND model = $2 AND enabled = true
                ORDER BY priority DESC 
                LIMIT 1
                "#,
                group_col
            )
        } else {
            format!(
                r#"
                SELECT priority 
                FROM abilities 
                WHERE {} = ? AND model = ? AND enabled = 1
                ORDER BY priority DESC 
                LIMIT 1
                "#,
                group_col
            )
        };

        println!(
            "ModelRouter: Querying priority with Group='{}', Model='{}'",
            group, model
        );

        let max_priority: Option<i64> = sqlx::query_scalar(&query)
            .bind(group)
            .bind(model)
            .fetch_optional(pool)
            .await?;

        let priority = match max_priority {
            Some(p) => p,
            None => return Ok(None), // No ability found
        };

        // 2. Get all candidates with this priority
        let query_candidates = if is_postgres {
            format!(
                r#"
                SELECT channel_id, weight 
                FROM abilities 
                WHERE {} = $1 AND model = $2 AND enabled = true AND priority = $3
                "#,
                group_col
            )
        } else {
            format!(
                r#"
                SELECT channel_id, weight 
                FROM abilities 
                WHERE {} = ? AND model = ? AND enabled = 1 AND priority = ?
                "#,
                group_col
            )
        };

        let candidates: Vec<(i32, i32)> = sqlx::query_as::<_, (i32, i64)>(&query_candidates)
            .bind(group)
            .bind(model)
            .bind(priority)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|(id, w)| (id, w as i32))
            .collect();

        if candidates.is_empty() {
            return Ok(None);
        }

        // 3. Weighted Random Selection
        let selected_channel_id = if candidates.len() == 1 {
            candidates[0].0
        } else {
            let total_weight: i32 = candidates.iter().map(|(_, w)| *w).sum();
            if total_weight <= 0 {
                // All weights are 0 or negative (invalid), pick random
                candidates[rand::thread_rng().gen_range(0..candidates.len())].0
            } else {
                let mut r = rand::thread_rng().gen_range(0..total_weight);
                let mut selected = candidates[0].0;
                for (id, weight) in candidates {
                    if weight <= 0 {
                        continue;
                    }
                    if r < weight {
                        selected = id;
                        break;
                    }
                    r -= weight;
                }
                selected
            }
        };

        // 4. Fetch Channel Details
        let channel_query = if is_postgres {
            r#"
            SELECT 
                id, type as "type_", key, status, name, weight, created_time, test_time, 
                response_time, base_url, models, "group", used_quota, model_mapping, 
                priority, auto_ban, other_info, tag, setting, param_override, 
                header_override, remark 
            FROM channels WHERE id = $1
            "#
        } else {
            r#"
            SELECT 
                id, type as type_, key, status, name, weight, created_time, test_time, 
                response_time, base_url, models, `group`, used_quota, model_mapping, 
                priority, auto_ban, other_info, tag, setting, param_override, 
                header_override, remark 
            FROM channels WHERE id = ?
            "#
        };

        let channel: Option<Channel> = sqlx::query_as(channel_query)
            .bind(selected_channel_id)
            .fetch_optional(pool)
            .await?;

        Ok(channel)
    }
}
