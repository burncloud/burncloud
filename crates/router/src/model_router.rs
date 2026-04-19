use anyhow::Result;
use burncloud_common::types::Channel;
use burncloud_database::sqlx;
use burncloud_database::Database;

use crate::channel_state::ChannelStateTracker;
use crate::exchange_rate::ExchangeRateService;
use crate::scheduler::{self, CombinedScheduler, SchedulerKind};

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
                FROM channel_abilities
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
                FROM channel_abilities
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
                FROM channel_abilities
                WHERE {} = $1 AND model = $2 AND enabled = true AND priority = $3
                "#,
                group_col
            )
        } else {
            format!(
                r#"
                SELECT channel_id, weight
                FROM channel_abilities
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
            .map(|(i, prefix)| format!("{prefix}{}", i + 1))
            .collect::<Vec<_>>()
            .join(", ");

        let channel_query = if is_postgres {
            format!(
                r#"
                SELECT
                    id, type as "type_", key, status, name, weight, created_time, test_time,
                    response_time, base_url, models, "group", used_quota, model_mapping,
                    priority, auto_ban, other_info, tag, setting, param_override,
                    header_override, remark, api_version, pricing_region
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
                    header_override, remark, api_version, pricing_region
                FROM channel_providers WHERE id IN ({})
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

    /// Route with multi-factor scheduler, returning ranked candidates (top-5).
    ///
    /// Uses PassthroughScheduler for groups without explicit policy.
    /// Returns at most 5 channels to limit failover attempts.
    ///
    /// Returns:
    /// - `Ok(vec)` with ranked channels (may be empty if model has no configuration)
    /// - `Err(NoAvailableChannelsError)` if channels exist but all are unavailable
    pub async fn route_with_scheduler(
        &self,
        group: &str,
        model: &str,
        state_tracker: &ChannelStateTracker,
        price_cache: &burncloud_service_billing::PriceCache,
        exchange_rate: &ExchangeRateService,
        scheduler_kind: Option<&SchedulerKind>,
    ) -> std::result::Result<Vec<Channel>, NoAvailableChannelsError> {
        let candidates = self.get_candidates(group, model).await.map_err(|e| {
            NoAvailableChannelsError {
                model: model.to_string(),
                reason: format!("Database error: {e}"),
            }
        })?;

        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        // Filter by availability
        let available: Vec<_> = candidates
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

        // Pick scheduler for this group
        let ranked = match scheduler_kind {
            Some(SchedulerKind::Combined { config }) => {
                let combined = CombinedScheduler::new(config.clone());
                let ctx = scheduler::build_context(
                    model,
                    &available,
                    state_tracker,
                    price_cache,
                    exchange_rate,
                )
                .await;
                scheduler::rank_candidates(available, &ctx, &combined)
            }
            _ => {
                // Passthrough fast path — no context building needed
                scheduler::rank_passthrough(available)
            }
        };

        // Limit to top-5 candidates
        let channels: Vec<Channel> = ranked
            .into_iter()
            .take(5)
            .map(|(ch, _)| ch)
            .collect();
        Ok(channels)
    }
}
