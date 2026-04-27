use std::str::FromStr;

use anyhow::Result;
use burncloud_common::types::Channel;
use burncloud_database::placeholder::{ph, phs};
use burncloud_database::sqlx;
use burncloud_database::Database;

use crate::affinity::{self, AffinityCache};
use crate::channel_state::ChannelStateTracker;
use crate::exchange_rate::ExchangeRateService;
use crate::scheduler::{self, CombinedScheduler, SchedulerKind, SchedulingRequest};

/// Which routing layer made the final channel-selection decision.
///
/// BGP analogy: `layer_decision` ~ BGP origin attribute — tells Grafana *where*
/// the route came from, not just *what* it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingDecision {
    /// L3 Affinity: HRW or cache hit hoisted the channel to rank-0.
    AffinityHit,
    /// L4 Scorer: CombinedScheduler ranked this channel first (no affinity hit).
    ScorerPicked,
    /// L5 Failover: attempt N (1-based) succeeded after earlier candidates failed.
    Failover { attempt: u32 },
}

impl RoutingDecision {
    /// Static label for non-dynamic variants (no heap allocation).
    pub fn as_label(&self) -> &'static str {
        match self {
            Self::AffinityHit => "affinity_hit",
            Self::ScorerPicked => "scorer_picked",
            Self::Failover { .. } => "failover",
        }
    }

    /// Full label including dynamic attempt number (e.g. `"failover_2"`).
    pub fn to_label(&self) -> String {
        match self {
            Self::Failover { attempt } => format!("failover_{attempt}"),
            _ => self.as_label().to_string(),
        }
    }
}

/// Error returned when no channels are available for a model.
#[derive(Debug, thiserror::Error)]
#[error("No available channels for model '{model}': {reason}")]
pub struct NoAvailableChannelsError {
    pub model: String,
    pub reason: String,
}

/// Aggregated inputs for [`ModelRouter::route_with_scheduler`].
///
/// Bundling request + services into one struct avoids parameter-list bloat
/// (clippy::too_many_arguments) as the scheduler pipeline grows (audit Part 4
/// 取舍 6 — `SchedulingContext` 字段膨胀预警).
pub struct RouteInputs<'a> {
    pub group: &'a str,
    pub model: &'a str,
    pub state_tracker: &'a ChannelStateTracker,
    pub price_cache: &'a burncloud_service_billing::PriceCache,
    pub exchange_rate: &'a ExchangeRateService,
    pub scheduler_kind: Option<&'a SchedulerKind>,
    pub request: &'a SchedulingRequest,
    pub affinity_cache: Option<&'a AffinityCache>,
}

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
        // Boolean literal differs by dialect: PG `true` vs SQLite `1`.
        let enabled_lit = if is_postgres { "true" } else { "1" };
        let query = format!(
            r#"
                SELECT priority
                FROM channel_abilities
                WHERE {col} = {p1} AND model = {p2} AND enabled = {enabled_lit}
                ORDER BY priority DESC
                LIMIT 1
            "#,
            col = group_col,
            p1 = ph(is_postgres, 1),
            p2 = ph(is_postgres, 2),
            enabled_lit = enabled_lit,
        );

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
        let query_candidates = format!(
            r#"
                SELECT channel_id, weight
                FROM channel_abilities
                WHERE {col} = {p1} AND model = {p2} AND enabled = {enabled_lit} AND priority = {p3}
            "#,
            col = group_col,
            p1 = ph(is_postgres, 1),
            p2 = ph(is_postgres, 2),
            p3 = ph(is_postgres, 3),
            enabled_lit = enabled_lit,
        );

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
        let placeholders = phs(is_postgres, channel_ids.len());

        // Identifier-quoting differs by dialect (`type as "type_"`, "group" vs `group`)
        // so the SELECT list is dialect-specific. Placeholders go through phs().
        let channel_query = if is_postgres {
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

    /// Route with the multi-factor scheduler, returning ranked candidates (top-5)
    /// and the routing-layer decision that determined the first candidate.
    ///
    /// Pipeline (audit decisions D6 / D7 / E-D1):
    ///
    /// ```text
    ///   get_candidates → availability filter → OrderType filter → Affinity (HRW + cache)
    ///     → Scorer (CombinedScheduler) → top-5 failover list
    /// ```
    ///
    /// The OrderType filter runs **before** Affinity so a Budget client never
    /// gets pinned to an expensive channel by historical affinity.
    ///
    /// Returns:
    /// - `Ok((vec, Some(decision)))` with ranked channels + which layer picked rank-0
    /// - `Ok((vec, None))` when channels exist but no model routing occurred (empty vec)
    /// - `Err(NoAvailableChannelsError)` if channels exist but all are unavailable,
    ///   **or** if `OrderType::Budget` filtered them all out (caller maps to 503).
    pub async fn route_with_scheduler(
        &self,
        inputs: RouteInputs<'_>,
    ) -> std::result::Result<(Vec<Channel>, Option<RoutingDecision>), NoAvailableChannelsError> {
        let RouteInputs {
            group,
            model,
            state_tracker,
            price_cache,
            exchange_rate,
            scheduler_kind,
            request,
            affinity_cache,
        } = inputs;

        let candidates = self.get_candidates(group, model).await.map_err(|e| {
            NoAvailableChannelsError {
                model: model.to_string(),
                reason: format!("Database error: {e}"),
            }
        })?;

        if candidates.is_empty() {
            return Ok((Vec::new(), None));
        }

        // L0: Filter by availability
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

        // L1.5: OrderType filter (Budget drops expensive channels here).
        // Pre-resolve per-channel prices to USD so the OrderType closure stays
        // sync. Honors `pricing_region`.
        let mut price_map: std::collections::HashMap<i32, i64> =
            std::collections::HashMap::with_capacity(available.len());
        for (ch, _) in &available {
            let region = ch.pricing_region.as_deref();
            if let Some(price) = price_cache
                .get(model, region.filter(|r| !r.is_empty()))
                .await
            {
                let raw = price.input_price.saturating_add(price.output_price);
                let nano = match burncloud_common::Currency::from_str(&price.currency) {
                    Ok(curr) if curr != burncloud_common::Currency::USD => {
                        let usd = exchange_rate.convert(
                            raw as f64,
                            curr,
                            burncloud_common::Currency::USD,
                        );
                        usd as i64
                    }
                    _ => raw,
                };
                price_map.insert(ch.id, nano);
            }
        }
        let price_of = |ch: &Channel| -> Option<i64> { price_map.get(&ch.id).copied() };

        let filtered = request.order_type.filter_candidates(available, price_of);
        if filtered.is_empty() {
            return Err(NoAvailableChannelsError {
                model: model.to_string(),
                reason: format!(
                    "OrderType {} filtered out all candidates (no channel meets price constraint)",
                    request.order_type.as_label()
                ),
            });
        }

        // L3: Affinity (Fast Path) — if cache hit and channel is in the
        // filtered set, promote it to the head of the ranked list. Affinity
        // does NOT replace ranking — it provides a strong hint that the
        // failover loop will still fall through if the affined channel fails.
        let affinity_pick: Option<i32> = match (affinity_cache, request.affinity_key()) {
            (Some(cache), Some(key)) => cache.lookup(key, model).or_else(|| {
                // Sticky TTL miss: fall through to HRW so we can refresh.
                affinity::pick_hrw(key, &filtered, |id| {
                    state_tracker.get_health_score(id, Some(model))
                })
            }),
            _ => None,
        };

        // L4: Scorer (Combined) — ranks the (filtered) candidate set.
        let mut ranked = match scheduler_kind {
            Some(SchedulerKind::Combined { config }) => {
                let combined = CombinedScheduler::new(config.clone());
                let ctx = scheduler::build_context(
                    model,
                    &filtered,
                    state_tracker,
                    price_cache,
                    exchange_rate,
                )
                .await;
                scheduler::rank_candidates(filtered, &ctx, &combined)
            }
            _ => {
                // Passthrough fast path — no context building needed
                scheduler::rank_passthrough(filtered)
            }
        };

        // Apply affinity preference: if `affinity_pick` is in `ranked`, hoist
        // it to the front and refresh the cache (stickiness reset).
        let mut affinity_hoisted = false;
        if let (Some(picked), Some(cache), Some(key)) =
            (affinity_pick, affinity_cache, request.affinity_key())
        {
            if let Some(pos) = ranked.iter().position(|(ch, _)| ch.id == picked) {
                if pos != 0 {
                    let entry = ranked.remove(pos);
                    ranked.insert(0, entry);
                }
                affinity_hoisted = true;
                cache.insert(key, model, picked);
                tracing::debug!(
                    user = key,
                    model,
                    channel = picked,
                    "affinity_hit"
                );
            }
        }

        // Limit to top-5 candidates
        let channels: Vec<Channel> = ranked
            .into_iter()
            .take(5)
            .map(|(ch, _)| ch)
            .collect();

        // Determine which layer made the final decision for rank-0.
        // Decision D8: do NOT split AffinityHit into CacheHit / HrwPick.
        let decision = if affinity_hoisted {
            Some(RoutingDecision::AffinityHit)
        } else {
            Some(RoutingDecision::ScorerPicked)
        };

        Ok((channels, decision))
    }
}
