use std::str::FromStr;

use anyhow::Result;
use burncloud_database::Database;
use burncloud_supply_channel::ChannelService;
use burncloud_supply_contracts::Channel;

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
    ///
    /// **Note:** For `Failover { attempt }` this returns the truncated `"failover"`
    /// without the attempt number. Use [`to_label()`](Self::to_label) for the
    /// full label (e.g. `"failover_2"`) when writing to `router_logs.layer_decision`.
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
    pub price_cache: &'a burncloud_commerce_billing::PriceCache,
    pub exchange_rate: &'a ExchangeRateService,
    pub scheduler_kind: Option<&'a SchedulerKind>,
    pub request: &'a SchedulingRequest,
    pub affinity_cache: Option<&'a AffinityCache>,
}

pub struct ModelRouter {
    db: std::sync::Arc<Database>,
    local: std::sync::Arc<crate::local_attachment::LocalAttachmentState>,
}

impl ModelRouter {
    /// 创建沿用 Supply 候选事实的 Router，并共享同数据库对象的本机准入状态。
    ///
    /// # 注意
    /// 应与本机适配器传入同一 Arc 的克隆；构造不执行连接检查或 I/O。
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn example(db: std::sync::Arc<burncloud_database::Database>) {
    /// let router = burncloud_router::model_router::ModelRouter::new(db);
    /// # }
    /// ```
    pub fn new(db: std::sync::Arc<Database>) -> Self {
        let local = crate::local_attachment::LocalAttachmentState::for_database(&db);
        Self { db, local }
    }

    pub(crate) fn local_attachment_state(
        &self,
    ) -> &std::sync::Arc<crate::local_attachment::LocalAttachmentState> {
        &self.local
    }

    /// Get all candidate channels for a model in a group.
    ///
    /// Returns channels sorted by priority (highest first) with their weights.
    pub async fn get_candidates(&self, group: &str, model: &str) -> Result<Vec<(Channel, i32)>> {
        // Supply owns the channel_abilities query and dialect details. Traffic keeps the
        // routing decision: map the highest-priority abilities to weighted Channel values.
        let abilities =
            ChannelService::list_enabled_at_highest_priority(self.db.as_ref(), group, model)
                .await?;
        if abilities.is_empty() {
            return Ok(Vec::new());
        }

        let candidates: Vec<(i32, i32)> = abilities
            .into_iter()
            .map(|ability| (ability.channel_id, ability.weight))
            .collect();
        let channel_ids: Vec<i32> = candidates.iter().map(|(id, _)| *id).collect();
        let channels = ChannelService::list_by_ids(self.db.as_ref(), &channel_ids).await?;

        let weight_map: std::collections::HashMap<i32, i32> = candidates.into_iter().collect();
        let result = channels
            .into_iter()
            .filter_map(|channel| weight_map.get(&channel.id).map(|&weight| (channel, weight)))
            .collect();

        Ok(result)
    }

    /// 按用户组策略返回候选快照；新策略不截 top-5，旧策略保持历史排序与亲和。
    ///
    /// # Examples
    /// ```no_run
    /// # async fn example(router: &burncloud_router::model_router::ModelRouter, inputs: burncloud_router::model_router::RouteInputs<'_>) {
    /// let candidates = router.route_with_scheduler(inputs).await;
    /// # }
    /// ```
    ///
    /// # Notes
    /// 空组或模型返回错误。候选不是派发许可，真实发送必须重新检查并发容量与心跳。
    /// 新策略不使用会话亲和；最高可用管理员档内，均衡按正权重分摊，稳定优先取最高
    /// 有限健康分，价格优先取最低可比较报价；同分、同价按权重分摊，缺价不视为免费。
    ///
    /// 显式旧策略保留历史流程（审计决定 D6 / D7 / E-D1）：
    ///
    /// ```text
    ///   最高优先级能力 → 可用性 → OrderType → 会话亲和（HRW 与缓存）
    ///     → 历史评分 → 最多五个故障切换候选
    /// ```
    ///
    /// OrderType 过滤先于会话亲和，避免预算请求被固定到超出价格上限的候选。
    /// 返回候选快照及首选的决策来源；供给无匹配记录时返回空列表，已有候选但无合格
    /// 首选时返回 NoAvailableChannelsError。此快照不能替代真实派发时的重新选择。
    #[allow(
        clippy::cognitive_complexity,
        reason = "single routing pipeline applying L0/L1.5/L3 filters in sequence; splitting requires a dedicated behavior-preserving refactor"
    )]
    pub async fn route_with_scheduler(
        &self,
        inputs: RouteInputs<'_>,
    ) -> std::result::Result<(Vec<Channel>, Option<RoutingDecision>), NoAvailableChannelsError>
    {
        if inputs.group.is_empty() || inputs.model.is_empty() {
            return Err(NoAvailableChannelsError {
                model: inputs.model.into(),
                reason: "group and model must not be empty".into(),
            });
        }
        if inputs.scheduler_kind.is_none_or(SchedulerKind::is_strict) {
            let (channels, plan) = self.strict_plan(&inputs).await?;
            let mut remaining: Vec<_> = channels;
            let mut ranked = Vec::new();
            while !remaining.is_empty() {
                match plan.pick(
                    remaining.iter().map(|ch| ch.id),
                    inputs.state_tracker,
                    |_| true,
                ) {
                    Ok(id) => {
                        let pos = remaining
                            .iter()
                            .position(|ch| ch.id == id)
                            .unwrap_or_else(|| unreachable!());
                        ranked.push(remaining.remove(pos));
                    }
                    Err(error) if ranked.is_empty() => {
                        return Err(NoAvailableChannelsError {
                            model: inputs.model.into(),
                            reason: error.code().into(),
                        })
                    }
                    Err(_) => break,
                }
            }
            return Ok((ranked, Some(RoutingDecision::ScorerPicked)));
        }
        self.route_legacy(inputs).await
    }

    // 保留旧排序和亲和路径，避免新策略的准入重选改变显式旧配置。
    #[allow(clippy::cognitive_complexity, reason = "保留既有线性旧策略行为")]
    async fn route_legacy(
        &self,
        inputs: RouteInputs<'_>,
    ) -> Result<(Vec<Channel>, Option<RoutingDecision>), NoAvailableChannelsError> {
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

        let candidates =
            self.get_candidates(group, model)
                .await
                .map_err(|e| NoAvailableChannelsError {
                    model: model.to_string(),
                    reason: format!("Database error: {e}"),
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
                let nano = match burncloud_commerce_contracts::pricing::Currency::from_str(
                    &price.currency,
                ) {
                    Ok(curr) if curr != burncloud_commerce_contracts::pricing::Currency::USD => {
                        match exchange_rate.convert(
                            raw as f64,
                            curr,
                            burncloud_commerce_contracts::pricing::Currency::USD,
                        ) {
                            Ok(usd) => usd as i64,
                            Err(e) => {
                                tracing::warn!("Currency conversion failed: {e}, using raw amount");
                                raw
                            }
                        }
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
                tracing::debug!(user = key, model, channel = picked, "affinity_hit");
            }
        }

        // Limit to top-5 candidates
        let channels: Vec<Channel> = ranked.into_iter().take(5).map(|(ch, _)| ch).collect();

        // Determine which layer made the final decision for rank-0.
        // Decision D8: do NOT split AffinityHit into CacheHit / HrwPick.
        let decision = if affinity_hoisted {
            Some(RoutingDecision::AffinityHit)
        } else {
            Some(RoutingDecision::ScorerPicked)
        };

        Ok((channels, decision))
    }
    async fn strict_plan(
        &self,
        inputs: &RouteInputs<'_>,
    ) -> Result<(Vec<Channel>, scheduler::StrictPlan), NoAvailableChannelsError> {
        let fail = |e: burncloud_database::DatabaseError| NoAvailableChannelsError {
            model: inputs.model.into(),
            reason: e.to_string(),
        };
        let abilities =
            ChannelService::list_enabled_for_model(&self.db, inputs.group, inputs.model)
                .await
                .map_err(fail)?;
        let ids: Vec<_> = abilities.iter().map(|a| a.channel_id).collect();
        let channels = ChannelService::list_by_ids(&self.db, &ids)
            .await
            .map_err(fail)?;
        let abilities: std::collections::HashMap<_, _> =
            abilities.into_iter().map(|a| (a.channel_id, a)).collect();
        let candidates: Vec<_> = channels
            .into_iter()
            .filter_map(|mut ch| {
                let ability = abilities.get(&ch.id)?;
                ch.priority = ability.priority;
                Some((ch, ability.weight))
            })
            .collect();
        let plan =
            scheduler::StrictPlan::new(inputs, &candidates, std::sync::Arc::clone(&self.local))
                .await;
        Ok((candidates.into_iter().map(|(ch, _)| ch).collect(), plan))
    }

    pub(crate) async fn route_for_dispatch(
        &self,
        inputs: RouteInputs<'_>,
    ) -> Result<
        (
            Vec<Channel>,
            Option<RoutingDecision>,
            Option<scheduler::StrictPlan>,
        ),
        NoAvailableChannelsError,
    > {
        if inputs.scheduler_kind.is_none_or(SchedulerKind::is_strict) {
            let (channels, plan) = self.strict_plan(&inputs).await?;
            Ok((channels, Some(RoutingDecision::ScorerPicked), Some(plan)))
        } else {
            let (channels, decision) = self.route_with_scheduler(inputs).await?;
            Ok((channels, decision, None))
        }
    }
}
