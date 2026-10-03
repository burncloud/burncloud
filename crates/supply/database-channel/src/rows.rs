//! Persistence representation for the channel tables.
//!
//! `burncloud-supply-contracts` owns the channel and ability business types and must not know a
//! database framework. These rows are the persistence side of the boundary:
//!
//! ```text
//! channel_providers  row --ChannelRow--> Channel  (domain, Supply contract)
//! channel_abilities  row --AbilityRow--> Ability
//! ```
//!
//! Visibility: `pub(crate)`. Callers of this crate see the Supply domain types, never the rows, so
//! the SQL representation cannot leak into a contract.

use burncloud_supply_contracts::{Ability, Channel};
use sqlx::FromRow;

/// `channel_providers` row.
#[derive(Debug, Clone, FromRow)]
pub(crate) struct ChannelRow {
    pub id: i32,
    /// Selected as `type as type_`; the domain type renames it back to `"type"` in JSON.
    pub type_: i32,
    pub key: String,
    pub status: i32,
    pub name: String,
    pub weight: i32,
    pub created_time: Option<i64>,
    pub test_time: Option<i64>,
    pub response_time: Option<i32>,
    pub base_url: Option<String>,
    pub models: String,
    /// Selected as `"group"` / `` `group` `` depending on the dialect.
    pub group: String,
    pub used_quota: i64,
    pub model_mapping: Option<String>,
    pub priority: i64,
    pub auto_ban: i32,
    pub other_info: Option<String>,
    pub tag: Option<String>,
    pub setting: Option<String>,
    pub param_override: Option<String>,
    pub header_override: Option<String>,
    pub remark: Option<String>,
    pub api_version: Option<String>,
    pub pricing_region: Option<String>,
    pub rpm_cap: Option<i32>,
    pub tpm_cap: Option<i64>,
    pub reservation_green: Option<f64>,
    pub reservation_yellow: Option<f64>,
    pub reservation_red: Option<f64>,
}

impl From<ChannelRow> for Channel {
    fn from(row: ChannelRow) -> Self {
        Self {
            id: row.id,
            type_: row.type_,
            key: row.key,
            status: row.status,
            name: row.name,
            weight: row.weight,
            created_time: row.created_time,
            test_time: row.test_time,
            response_time: row.response_time,
            base_url: row.base_url,
            models: row.models,
            group: row.group,
            used_quota: row.used_quota,
            model_mapping: row.model_mapping,
            priority: row.priority,
            auto_ban: row.auto_ban,
            other_info: row.other_info,
            tag: row.tag,
            setting: row.setting,
            param_override: row.param_override,
            header_override: row.header_override,
            remark: row.remark,
            api_version: row.api_version,
            pricing_region: row.pricing_region,
            rpm_cap: row.rpm_cap,
            tpm_cap: row.tpm_cap,
            reservation_green: row.reservation_green,
            reservation_yellow: row.reservation_yellow,
            reservation_red: row.reservation_red,
        }
    }
}

impl From<Channel> for ChannelRow {
    fn from(channel: Channel) -> Self {
        Self {
            id: channel.id,
            type_: channel.type_,
            key: channel.key,
            status: channel.status,
            name: channel.name,
            weight: channel.weight,
            created_time: channel.created_time,
            test_time: channel.test_time,
            response_time: channel.response_time,
            base_url: channel.base_url,
            models: channel.models,
            group: channel.group,
            used_quota: channel.used_quota,
            model_mapping: channel.model_mapping,
            priority: channel.priority,
            auto_ban: channel.auto_ban,
            other_info: channel.other_info,
            tag: channel.tag,
            setting: channel.setting,
            param_override: channel.param_override,
            header_override: channel.header_override,
            remark: channel.remark,
            api_version: channel.api_version,
            pricing_region: channel.pricing_region,
            rpm_cap: channel.rpm_cap,
            tpm_cap: channel.tpm_cap,
            reservation_green: channel.reservation_green,
            reservation_yellow: channel.reservation_yellow,
            reservation_red: channel.reservation_red,
        }
    }
}

/// `channel_abilities` row.
#[derive(Debug, Clone, FromRow)]
pub(crate) struct AbilityRow {
    /// Selected as `"group"` / `` `group` `` depending on the dialect.
    pub group: String,
    pub model: String,
    pub channel_id: i32,
    pub enabled: i32,
    pub priority: i64,
    pub weight: i32,
}

impl From<AbilityRow> for Ability {
    fn from(row: AbilityRow) -> Self {
        Self {
            group: row.group,
            model: row.model,
            channel_id: row.channel_id,
            enabled: row.enabled,
            priority: row.priority,
            weight: row.weight,
        }
    }
}

impl From<Ability> for AbilityRow {
    fn from(ability: Ability) -> Self {
        Self {
            group: ability.group,
            model: ability.model,
            channel_id: ability.channel_id,
            enabled: ability.enabled,
            priority: ability.priority,
            weight: ability.weight,
        }
    }
}
