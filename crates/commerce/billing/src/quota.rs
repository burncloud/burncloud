//! Monthly-quota billing plans and subscriptions, mapped onto the shipped schema.
//!
//! Migration `0015_monthly_quota.sql` (in `burncloud-database`) creates `billing_plans` and
//! `billing_subscriptions`, but no Rust type modelled them: `BillingPlan`, `BillingPlanInput`,
//! `BillingMode`, `BillingSubscription`, `SubscriptionStatus` and `SubscriptionStatusResponse`
//! had zero hits across `crates/**`, so those columns were reachable only through hand-written SQL.
//!
//! This module is the adapter half of that mapping, and it belongs in this crate rather than in
//! `burncloud-commerce-contracts`: that package is a pure pricing contract which "knows neither a
//! database framework nor logging", as its own module header states, and it explicitly leaves "the
//! row structs, their SQL row mapping and the conversions into these domain types" to
//! `burncloud-commerce-billing`.
//!
//! # Units
//!
//! `monthly_fee` is a CNY amount in nanodollars, the same scale as
//! [`burncloud_commerce_contracts::price_u64::NANO_PER_DOLLAR`]. No arithmetic is performed on it
//! here, so this module introduces no second pricing semantics.
//!
//! # Deliberately out of scope
//!
//! The ruling on which `quota_limit` is authoritative (`router_tokens` versus
//! `billing_subscriptions`) and the billing admission path both stay deferred, as issue #652
//! records. These types describe the schema; nothing in this module enforces a quota.
//!
//! # Known dialect divergence (reported, not worked around)
//!
//! `billing_subscriptions.user_id` is `TEXT` in the PostgreSQL migration but is still `INTEGER` in
//! the SQLite one, while `users.id` is `TEXT PRIMARY KEY` in both. The PostgreSQL side was already
//! corrected under #833 ("Historical fresh installs previously failed while creating this FK"); the
//! SQLite declaration was not. This module maps the column to [`String`], matching the foreign key's
//! target type in both dialects and the PostgreSQL column. On SQLite, INTEGER affinity coerces a
//! digit-only user id to an integer, so such an id cannot round-trip there. That is a schema defect
//! for a follow-up migration; these types must not paper over it.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A text column value outside the domain its migration comment declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownValueError {
    field: &'static str,
    value: String,
}

impl UnknownValueError {
    /// The column whose value was rejected.
    #[must_use]
    pub fn field(&self) -> &'static str {
        self.field
    }

    /// The rejected text, verbatim.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for UnknownValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "column {} carries unsupported value {:?}",
            self.field, self.value
        )
    }
}

impl std::error::Error for UnknownValueError {}

/// `billing_plans.billing_mode` — the migration comment permits exactly these two values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingMode {
    /// `'per_request'`: `request_limit` carries the plan's quota.
    PerRequest,
    /// `'per_token'`: `token_limit` carries the plan's quota.
    PerToken,
}

impl BillingMode {
    /// The exact text stored in the `billing_mode` column.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PerRequest => "per_request",
            Self::PerToken => "per_token",
        }
    }
}

impl FromStr for BillingMode {
    type Err = UnknownValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "per_request" => Ok(Self::PerRequest),
            "per_token" => Ok(Self::PerToken),
            other => Err(UnknownValueError {
                field: "billing_mode",
                value: other.to_owned(),
            }),
        }
    }
}

/// `billing_subscriptions.status` — the migration comment permits exactly these three values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    /// `'active'`: the column default; the subscription is in force until it expires.
    Active,
    /// `'expired'`: the subscription passed `expires_at`.
    Expired,
    /// `'cancelled'`: the subscription was cancelled and is unusable regardless of remaining quota.
    Cancelled,
}

impl SubscriptionStatus {
    /// The exact text stored in the `status` column.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for SubscriptionStatus {
    type Err = UnknownValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "active" => Ok(Self::Active),
            "expired" => Ok(Self::Expired),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(UnknownValueError {
                field: "status",
                value: other.to_owned(),
            }),
        }
    }
}

/// A row of `billing_plans`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BillingPlan {
    /// `id`, assigned by the database.
    pub id: i64,
    /// `name`, unique across plans.
    pub name: String,
    /// `monthly_fee`: CNY nanodollars (9 decimal places).
    pub monthly_fee: i64,
    /// `billing_mode`, decoded into its two permitted values.
    pub billing_mode: BillingMode,
    /// `request_limit`; only meaningful for [`BillingMode::PerRequest`], otherwise `NULL`.
    pub request_limit: Option<i64>,
    /// `token_limit`; only meaningful for [`BillingMode::PerToken`], otherwise `NULL`.
    pub token_limit: Option<i64>,
    /// `channel_id`: the upstream channel the plan is bound to.
    pub channel_id: i64,
    /// `created_at`, in milliseconds since the Unix epoch.
    pub created_at: i64,
    /// `updated_at`, in milliseconds since the Unix epoch.
    pub updated_at: i64,
}

/// The `billing_plans` columns an insert supplies; `id` and the timestamps are database-generated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BillingPlanInput {
    /// `name`, unique across plans.
    pub name: String,
    /// `monthly_fee`: CNY nanodollars (9 decimal places).
    pub monthly_fee: i64,
    /// `billing_mode`; decides which limit column is meaningful.
    pub billing_mode: BillingMode,
    /// `request_limit` for a per-request plan.
    pub request_limit: Option<i64>,
    /// `token_limit` for a per-token plan.
    pub token_limit: Option<i64>,
    /// `channel_id`: the upstream channel the plan is bound to.
    pub channel_id: i64,
}

/// A row of `billing_subscriptions`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BillingSubscription {
    /// `id`, assigned by the database.
    pub id: i64,
    /// `user_id`, a [`String`] because it references the text `users.id` key.
    pub user_id: String,
    /// `plan_id`, referencing `billing_plans.id`.
    pub plan_id: i64,
    /// `channel_id`, inherited from the plan.
    pub channel_id: i64,
    /// `status`, decoded into its three permitted values.
    pub status: SubscriptionStatus,
    /// `quota_used`: requests or tokens consumed, per the migration comment.
    pub quota_used: i64,
    /// `quota_limit`: the granted quota, in the same unit as `quota_used`.
    pub quota_limit: i64,
    /// `expires_at`, in milliseconds since the Unix epoch.
    pub expires_at: i64,
    /// `created_at`, in milliseconds since the Unix epoch.
    pub created_at: i64,
    /// `updated_at`, in milliseconds since the Unix epoch.
    pub updated_at: i64,
}

impl BillingSubscription {
    /// Quota left, never negative: an over-quota row reports `0`, not a negative remainder.
    #[must_use]
    pub fn remaining_quota(&self) -> i64 {
        // Saturating subtraction: the two columns are independent i64 values from the database, so
        // an extreme pair must clamp rather than overflow.
        self.quota_limit.saturating_sub(self.quota_used).max(0)
    }

    /// True when the granted quota is used up or exceeded.
    #[must_use]
    pub fn is_exhausted(&self) -> bool {
        self.quota_used >= self.quota_limit
    }

    /// True once `expires_at` has been reached; the boundary itself counts as expired.
    #[must_use]
    pub fn is_expired_at(&self, now_ms: i64) -> bool {
        now_ms >= self.expires_at
    }

    /// True when this subscription may still serve traffic: active, unexpired and not exhausted.
    #[must_use]
    pub fn is_usable_at(&self, now_ms: i64) -> bool {
        self.status == SubscriptionStatus::Active
            && !self.is_expired_at(now_ms)
            && !self.is_exhausted()
    }

    /// Projects this row into the status view for a caller at `now_ms`.
    #[must_use]
    pub fn status_response_at(&self, now_ms: i64) -> SubscriptionStatusResponse {
        SubscriptionStatusResponse {
            status: self.status,
            quota_used: self.quota_used,
            quota_limit: self.quota_limit,
            remaining_quota: self.remaining_quota(),
            expires_at: self.expires_at,
            usable: self.is_usable_at(now_ms),
        }
    }
}

/// The status projection of a [`BillingSubscription`], with no enforcement attached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionStatusResponse {
    /// The stored `status`.
    pub status: SubscriptionStatus,
    /// `quota_used`.
    pub quota_used: i64,
    /// `quota_limit`.
    pub quota_limit: i64,
    /// Quota left, floored at zero.
    pub remaining_quota: i64,
    /// `expires_at`, in milliseconds since the Unix epoch.
    pub expires_at: i64,
    /// Whether the subscription may still serve traffic at the time of the call.
    pub usable: bool,
}

/// A raw `billing_plans` row as sqlx decodes it, before the text columns become enums.
#[derive(Debug, Clone, FromRow)]
pub struct BillingPlanRow {
    /// `id`.
    pub id: i64,
    /// `name`.
    pub name: String,
    /// `monthly_fee`.
    pub monthly_fee: i64,
    /// `billing_mode`, still text.
    pub billing_mode: String,
    /// `request_limit`.
    pub request_limit: Option<i64>,
    /// `token_limit`.
    pub token_limit: Option<i64>,
    /// `channel_id`.
    pub channel_id: i64,
    /// `created_at`.
    pub created_at: i64,
    /// `updated_at`.
    pub updated_at: i64,
}

impl TryFrom<BillingPlanRow> for BillingPlan {
    type Error = UnknownValueError;

    fn try_from(row: BillingPlanRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            name: row.name,
            monthly_fee: row.monthly_fee,
            billing_mode: BillingMode::from_str(&row.billing_mode)?,
            request_limit: row.request_limit,
            token_limit: row.token_limit,
            channel_id: row.channel_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// A raw `billing_subscriptions` row as sqlx decodes it.
#[derive(Debug, Clone, FromRow)]
pub struct BillingSubscriptionRow {
    /// `id`.
    pub id: i64,
    /// `user_id`, matching the text `users.id` key.
    pub user_id: String,
    /// `plan_id`.
    pub plan_id: i64,
    /// `channel_id`.
    pub channel_id: i64,
    /// `status`, still text.
    pub status: String,
    /// `quota_used`.
    pub quota_used: i64,
    /// `quota_limit`.
    pub quota_limit: i64,
    /// `expires_at`.
    pub expires_at: i64,
    /// `created_at`.
    pub created_at: i64,
    /// `updated_at`.
    pub updated_at: i64,
}

impl TryFrom<BillingSubscriptionRow> for BillingSubscription {
    type Error = UnknownValueError;

    fn try_from(row: BillingSubscriptionRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            user_id: row.user_id,
            plan_id: row.plan_id,
            channel_id: row.channel_id,
            status: SubscriptionStatus::from_str(&row.status)?,
            quota_used: row.quota_used,
            quota_limit: row.quota_limit,
            expires_at: row.expires_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}
