use anyhow::Result;
use burncloud_common::types::Channel;
use burncloud_database::sqlx;
use burncloud_database::Database;
use rand::Rng; // Use re-exported sqlx

use crate::channel_state::ChannelStateTracker;

/// Error returned when no channels are available for a model.
#[derive(Debug)]
pub struct NoAvailableChannelsError {
    pub model: String,
    pub reason: String,
}

impl std::fmt::Display for NoAvailableChannelsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "No available channels for model '{}': {}",
            self.model, self.reason
        )
    }
}

impl std::error::Error for NoAvailableChannelsError {}

pub struct ModelRouter {
    db: std::sync::Arc<Database>,
}

impl ModelRouter {
    pub fn new(db: std::sync::Arc<Database>) -> Self {
        Self { db }
    }

    /// Get all candidate channels for a model in a group.
    ///
    /// Returns channels sorted by priority (highest first) with their weights.
    pub async fn get_candidates(&self, group: &str, model: &str) -> Result<Vec<(Channel, i32)>> {
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

        let max_priority: Option<i64> = sqlx::query_scalar(&query)
            .bind(group)
            .bind(model)
            .fetch_optional(pool)
            .await?;

        let priority = match max_priority {
            Some(p) => p,
            None => return Ok(Vec::new()), // No ability found
        };

        // 2. Get all candidate channel IDs with weights
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
            return Ok(Vec::new());
        }

        // 3. Fetch Channel Details for all candidates
        let channel_ids: Vec<i32> = candidates.iter().map(|(id, _)| *id).collect();
        let placeholders: String = channel_ids
            .iter()
            .map(|_| if is_postgres { "$" } else { "?" })
            .enumerate()
            .map(|(i, prefix)| format!("{}{}", prefix, i + 1))
            .collect::<Vec<_>>()
            .join(", ");

        let channel_query = if is_postgres {
            format!(
                r#"
                SELECT
                    id, type as "type_", key, status, name, weight, created_time, test_time,
                    response_time, base_url, models, "group", used_quota, model_mapping,
                    priority, auto_ban, other_info, tag, setting, param_override,
                    header_override, remark, api_version
                FROM channels WHERE id IN ({})
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
                    header_override, remark, api_version
                FROM channels WHERE id IN ({})
                "#,
                placeholders
            )
        };

        let mut query = sqlx::query_as::<_, Channel>(&channel_query);
        for &id in &channel_ids {
            query = query.bind(id);
        }

        let channels: Vec<Channel> = query.fetch_all(pool).await?;

        // 4. Map channels to weights
        let weight_map: std::collections::HashMap<i32, i32> = candidates.into_iter().collect();

        let result: Vec<(Channel, i32)> = channels
            .into_iter()
            .filter_map(|ch| weight_map.get(&ch.id).map(|&w| (ch, w)))
            .collect();

        Ok(result)
    }

    /// Routes a request to a suitable channel with state filtering.
    ///
    /// This method filters out unavailable channels based on the ChannelStateTracker
    /// and selects from the remaining healthy channels using weighted random selection.
    ///
    /// # Arguments
    /// * `group` - The user group for routing
    /// * `model` - The model to route to
    /// * `state_tracker` - The channel state tracker for health filtering
    ///
    /// # Returns
    /// * `Ok(Some(channel))` - A healthy channel was selected
    /// * `Ok(None)` - No channels configured for this model/group
    /// * `Err(NoAvailableChannelsError)` - Channels exist but none are available
    pub async fn route_with_state(
        &self,
        group: &str,
        model: &str,
        state_tracker: &ChannelStateTracker,
    ) -> std::result::Result<Option<Channel>, NoAvailableChannelsError> {
        let candidates =
            self.get_candidates(group, model)
                .await
                .map_err(|e| NoAvailableChannelsError {
                    model: model.to_string(),
                    reason: format!("Database error: {}", e),
                })?;

        if candidates.is_empty() {
            return Ok(None);
        }

        // Filter by availability using state tracker
        let available: Vec<(Channel, i32)> = candidates
            .into_iter()
            .filter(|(ch, _)| state_tracker.is_available(ch.id, Some(model)))
            .collect();

        if available.is_empty() {
            return Err(NoAvailableChannelsError {
                model: model.to_string(),
                reason: "All channels are currently unavailable (rate limited, auth failed, or exhausted)"
                    .to_string(),
            });
        }

        // Sort by health score (channels with more success and less failures first)
        let mut sorted = available;
        sorted.sort_by(|(a, _), (b, _)| {
            // Calculate health score: higher is better
            // This is a simple heuristic - could be improved with more sophisticated scoring
            let score_a = self.calculate_health_score(a.id, state_tracker, model);
            let score_b = self.calculate_health_score(b.id, state_tracker, model);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Weighted Random Selection from available channels
        let selected_channel = if sorted.len() == 1 {
            sorted[0].0.clone()
        } else {
            let total_weight: i32 = sorted.iter().map(|(_, w)| *w).sum();
            if total_weight <= 0 {
                sorted[rand::thread_rng().gen_range(0..sorted.len())]
                    .0
                    .clone()
            } else {
                let mut r = rand::thread_rng().gen_range(0..total_weight);
                let mut selected = sorted[0].0.clone();
                for (ch, weight) in &sorted {
                    if *weight <= 0 {
                        continue;
                    }
                    if r < *weight {
                        selected = ch.clone();
                        break;
                    }
                    r -= weight;
                }
                selected
            }
        };

        Ok(Some(selected_channel))
    }

    /// Calculate a health score for a channel.
    ///
    /// Higher scores indicate healthier channels.
    /// Considers success rate, average latency, and current status.
    fn calculate_health_score(
        &self,
        channel_id: i32,
        state_tracker: &ChannelStateTracker,
        model: &str,
    ) -> f64 {
        state_tracker.get_health_score(channel_id, Some(model))
    }
}
