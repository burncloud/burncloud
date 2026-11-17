use super::{ChannelStateTracker, OrderType, SchedulerKind};
use crate::local_attachment::LocalAttachmentState;
use burncloud_commerce_contracts::pricing::Currency;
use burncloud_supply_contracts::Channel;
use rand::Rng;
use std::{collections::HashMap, str::FromStr, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionError {
    Unavailable,
    NoComparablePrice,
    OrderType,
}

impl SelectionError {
    pub(crate) fn rejected_by(self) -> &'static str {
        if self == Self::OrderType {
            "order_type"
        } else {
            "scheduler"
        }
    }

    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Unavailable | Self::OrderType => "no_available_channels",
            Self::NoComparablePrice => "no_comparable_price",
        }
    }
}

struct Candidate {
    priority: i64,
    weight: u32,
    price: Option<i128>,
    local_url: Option<String>,
    order_allowed: bool,
    enabled: bool,
}

pub(crate) struct StrictPlan {
    kind: SchedulerKind,
    model: String,
    candidates: HashMap<i32, Candidate>,
    local: Arc<LocalAttachmentState>,
}

impl StrictPlan {
    pub(crate) async fn new(
        inputs: &crate::model_router::RouteInputs<'_>,
        channels: &[(Channel, i32)],
        local: Arc<LocalAttachmentState>,
    ) -> Self {
        let rates = inputs.exchange_rate.snapshot_nano_rates();
        let mut quotes = HashMap::new();
        let mut candidates = HashMap::new();
        for (channel, weight) in channels {
            if *weight <= 0 {
                tracing::warn!(
                    channel_id = channel.id,
                    weight,
                    "nonpositive scheduler weight excluded"
                );
                continue;
            }
            let region = channel.pricing_region.as_deref().filter(|s| !s.is_empty());
            let price = if let Some(quote) = quotes.get(&region.map(str::to_owned)) {
                *quote
            } else {
                let quote = inputs
                    .price_cache
                    .get(inputs.model, region)
                    .await
                    .and_then(|p| {
                        reference_price(p.input_price, p.output_price, &p.currency, &rates)
                    });
                quotes.insert(region.map(str::to_owned), quote);
                quote
            };
            let order_allowed = match inputs.request.order_type {
                OrderType::Budget {
                    max_price_nanodollars,
                } => price.map_or(max_price_nanodollars == i64::MAX, |p| {
                    p <= i128::from(max_price_nanodollars)
                }),
                _ => true,
            };
            candidates.insert(
                channel.id,
                Candidate {
                    order_allowed,
                    enabled: channel.status == 1,
                    priority: channel.priority,
                    weight: *weight as u32,
                    price,
                    local_url: (channel.tag.as_deref() == Some("burncloud-node-local"))
                        .then(|| channel.base_url.clone().unwrap_or_default()),
                },
            );
        }
        Self {
            kind: inputs
                .scheduler_kind
                .cloned()
                .unwrap_or(SchedulerKind::Balanced),
            model: inputs.model.to_owned(),
            candidates,
            local,
        }
    }

    pub(crate) fn pick(
        &self,
        ids: impl Iterator<Item = i32>,
        tracker: &ChannelStateTracker,
        available: impl Fn(i32) -> bool,
    ) -> Result<i32, SelectionError> {
        let mut pool = Vec::new();
        for id in ids {
            let Some(entry) = self.candidates.get(&id) else {
                continue;
            };
            if !entry.enabled
                || !tracker.is_available(id, Some(&self.model))
                || !available(id)
                || entry
                    .local_url
                    .as_deref()
                    .is_some_and(|url| !self.local.lifecycle_allows(id, &self.model, url))
            {
                continue;
            }
            let health = tracker.get_health_score(id, Some(&self.model));
            if matches!(self.kind, SchedulerKind::StabilityFirst) && !health.is_finite() {
                tracing::warn!(channel_id = id, "nonfinite health score excluded");
                continue;
            }
            pool.push((id, entry, health));
        }
        if !pool.is_empty() && pool.iter().all(|(_, entry, _)| !entry.order_allowed) {
            return Err(SelectionError::OrderType);
        }
        pool.retain(|(_, entry, _)| entry.order_allowed);
        let highest = pool
            .iter()
            .map(|(_, entry, _)| entry.priority)
            .max()
            .ok_or(SelectionError::Unavailable)?;
        pool.retain(|(_, entry, _)| entry.priority == highest);
        match self.kind {
            SchedulerKind::StabilityFirst => {
                let best = pool
                    .iter()
                    .map(|(_, _, score)| *score)
                    .fold(f64::NEG_INFINITY, f64::max);
                pool.retain(|(_, _, score)| *score == best);
            }
            SchedulerKind::PriceFirst => {
                let best = pool
                    .iter()
                    .filter_map(|(_, entry, _)| entry.price)
                    .min()
                    .ok_or(SelectionError::NoComparablePrice)?;
                pool.retain(|(_, entry, _)| entry.price == Some(best));
            }
            _ => {}
        }
        let weighted: Vec<_> = pool
            .iter()
            .map(|(id, entry, _)| (*id, entry.weight))
            .collect();
        Ok(weighted_pick(&weighted, &mut rand::thread_rng()))
    }
}

fn weighted_pick(candidates: &[(i32, u32)], rng: &mut impl Rng) -> i32 {
    if candidates.len() == 1 {
        return candidates[0].0;
    }
    let total: u128 = candidates
        .iter()
        .map(|(_, weight)| u128::from(*weight))
        .sum();
    let mut draw = rng.gen_range(0..total);
    for &(id, weight) in candidates {
        if draw < u128::from(weight) {
            return id;
        }
        draw -= u128::from(weight);
    }
    unreachable!("正权重候选的随机区间必须覆盖抽样值")
}

fn reference_price(
    input: i64,
    output: i64,
    currency: &str,
    rates: &HashMap<(Currency, Currency), i64>,
) -> Option<i128> {
    if input < 0 || output < 0 {
        return None;
    }
    let currency = Currency::from_str(currency).ok()?;
    let sum = i128::from(input).checked_add(i128::from(output))?;
    if currency == Currency::USD {
        return Some(sum);
    }
    let (numerator, denominator) = if let Some(&rate) = rates.get(&(currency, Currency::USD)) {
        if rate <= 0 {
            return None;
        }
        (sum.checked_mul(i128::from(rate))?, 1_000_000_000_i128)
    } else {
        let rate = *rates.get(&(Currency::USD, currency))?;
        if rate <= 0 {
            return None;
        }
        (sum.checked_mul(1_000_000_000)?, i128::from(rate))
    };
    numerator
        .checked_add(denominator / 2)?
        .checked_div(denominator)
}

#[cfg(test)]
#[path = "strict_tests.rs"]
mod tests;
