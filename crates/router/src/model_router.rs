use burncloud_database::Database;
use burncloud_common::types::Channel;
use rand::Rng;
use anyhow::Result;
use burncloud_database::sqlx::{self, Row}; // Use re-exported sqlx

pub struct ModelRouter {
    db: std::sync::Arc<Database>,
}
// ... (rest of code)
// No changes needed for sqlx::query usage if `use burncloud_database::sqlx;` is present,
// but since I used `sqlx::query` in the body, I need `use burncloud_database::sqlx` to bring the module into scope as `sqlx`.

impl ModelRouter {
    pub fn new(db: std::sync::Arc<Database>) -> Self {
        Self { db }
    }

    /// Routes a request to a suitable channel based on user group and requested model.
    /// 
    /// Logic:
    /// 1. Find all enabled abilities matching (group, model).
    /// 2. Group them by priority (High priority first).
    /// 3. Select the highest priority tier.
    /// 4. Within that tier, use weighted random selection to pick a channel.
    /// 5. Return the full Channel details.
    pub async fn route(&self, group: &str, model: &str) -> Result<Option<Channel>> {
        let conn = self.db.get_connection()?;
        let pool = conn.pool();

        // 1. Get max priority for this group/model pair
        // Note: "group" is a keyword, quoted as "group" (Postgres) or `group` (SQLite)
        // We need to handle dialect difference or use simple SQL compatible with both if possible.
        // Since our Schema uses quotes for Postgres and backticks for SQLite, accessing it might be tricky via raw SQL 
        // if we don't know the dialect here.
        // However, `Database` knows the kind.
        
        let group_col = if self.db.kind() == "postgres" { "\"group\"" } else { "`group`" };

        let query = format!(
            r#"
            SELECT priority 
            FROM abilities 
            WHERE {} = ? AND model = ? AND enabled = 1
            ORDER BY priority DESC 
            LIMIT 1
            "#,
            group_col
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
        let query_candidates = format!(
            r#"
            SELECT channel_id, weight 
            FROM abilities 
            WHERE {} = ? AND model = ? AND enabled = 1 AND priority = ?
            "#,
            group_col
        );

        let candidates: Vec<(i32, i32)> = sqlx::query_as::<_, (i32, i64)>(&query_candidates) // weight is stored as int in DB, read as i64
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
                    if weight <= 0 { continue; }
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
        // We use the standard columns defined in our Schema
        let channel_query = match self.db.kind().as_str() {
            "postgres" => r#"
                SELECT 
                    id, type as "type_", key, status, name, weight, created_time, test_time, 
                    response_time, base_url, models, "group", used_quota, model_mapping, 
                    priority, auto_ban, other_info, tag, setting, param_override, 
                    header_override, remark 
                FROM channels WHERE id = ?
            "#,
            _ => r#"
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
