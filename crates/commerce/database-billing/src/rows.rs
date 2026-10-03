//! Persistence representation for the billing tables.
//!
//! `burncloud-commerce-contracts` owns the price/currency business types and must not know a
//! database framework. These row structs are the persistence side of the boundary:
//!
//! ```text
//! billing_prices row ──PriceRow──► Price   (domain, Commerce contract)
//! billing_tiered_prices row ──TieredPriceRow──► TieredPrice
//! billing_exchange_rates row ──ExchangeRateRow──► ExchangeRate
//! ```
//!
//! Field names, SQL column order and the `#[sqlx(default)]` NULL handling are identical to the
//! table definitions, so switching the queries to these structs does not change what is read or
//! written. The conversion is field-by-field; `tests/row_domain_parity.rs` proves no field is
//! dropped or reordered.
//!
//! `ExchangeRateRow` is defined for symmetry and for the repository task that reads
//! `billing_exchange_rates`; today that table is read by Traffic's `ExchangeRateService`, so no
//! query in this crate uses it yet.

use burncloud_commerce_contracts::pricing::{ExchangeRate, Price, TieredPrice};
use sqlx::FromRow;

/// `billing_prices` row.
#[derive(Debug, Clone, FromRow)]
pub struct PriceRow {
    pub id: i32,
    pub model: String,
    pub currency: String,
    pub input_price: i64,
    pub output_price: i64,
    pub cache_read_input_price: Option<i64>,
    pub cache_creation_input_price: Option<i64>,
    pub batch_input_price: Option<i64>,
    pub batch_output_price: Option<i64>,
    pub priority_input_price: Option<i64>,
    pub priority_output_price: Option<i64>,
    pub audio_input_price: Option<i64>,
    pub audio_output_price: Option<i64>,
    pub reasoning_price: Option<i64>,
    pub embedding_price: Option<i64>,
    pub image_price: Option<i64>,
    pub video_price: Option<i64>,
    pub music_price: Option<i64>,
    pub source: Option<String>,
    pub region: Option<String>,
    pub context_window: Option<i64>,
    pub max_output_tokens: Option<i64>,
    /// Stored as INTEGER 0/1 in the database.
    #[sqlx(default)]
    pub supports_vision: Option<i32>,
    /// Stored as INTEGER 0/1 in the database.
    #[sqlx(default)]
    pub supports_function_calling: Option<i32>,
    pub synced_at: Option<i64>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    #[sqlx(default)]
    pub voices_pricing: Option<String>,
    #[sqlx(default)]
    pub video_pricing: Option<String>,
    #[sqlx(default)]
    pub asr_pricing: Option<String>,
    #[sqlx(default)]
    pub realtime_pricing: Option<String>,
    #[sqlx(default)]
    pub model_type: Option<String>,
}

impl From<PriceRow> for Price {
    fn from(row: PriceRow) -> Self {
        Self {
            id: row.id,
            model: row.model,
            currency: row.currency,
            input_price: row.input_price,
            output_price: row.output_price,
            cache_read_input_price: row.cache_read_input_price,
            cache_creation_input_price: row.cache_creation_input_price,
            batch_input_price: row.batch_input_price,
            batch_output_price: row.batch_output_price,
            priority_input_price: row.priority_input_price,
            priority_output_price: row.priority_output_price,
            audio_input_price: row.audio_input_price,
            audio_output_price: row.audio_output_price,
            reasoning_price: row.reasoning_price,
            embedding_price: row.embedding_price,
            image_price: row.image_price,
            video_price: row.video_price,
            music_price: row.music_price,
            source: row.source,
            region: row.region,
            context_window: row.context_window,
            max_output_tokens: row.max_output_tokens,
            supports_vision: row.supports_vision,
            supports_function_calling: row.supports_function_calling,
            synced_at: row.synced_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
            voices_pricing: row.voices_pricing,
            video_pricing: row.video_pricing,
            asr_pricing: row.asr_pricing,
            realtime_pricing: row.realtime_pricing,
            model_type: row.model_type,
        }
    }
}

impl From<Price> for PriceRow {
    fn from(price: Price) -> Self {
        Self {
            id: price.id,
            model: price.model,
            currency: price.currency,
            input_price: price.input_price,
            output_price: price.output_price,
            cache_read_input_price: price.cache_read_input_price,
            cache_creation_input_price: price.cache_creation_input_price,
            batch_input_price: price.batch_input_price,
            batch_output_price: price.batch_output_price,
            priority_input_price: price.priority_input_price,
            priority_output_price: price.priority_output_price,
            audio_input_price: price.audio_input_price,
            audio_output_price: price.audio_output_price,
            reasoning_price: price.reasoning_price,
            embedding_price: price.embedding_price,
            image_price: price.image_price,
            video_price: price.video_price,
            music_price: price.music_price,
            source: price.source,
            region: price.region,
            context_window: price.context_window,
            max_output_tokens: price.max_output_tokens,
            supports_vision: price.supports_vision,
            supports_function_calling: price.supports_function_calling,
            synced_at: price.synced_at,
            created_at: price.created_at,
            updated_at: price.updated_at,
            voices_pricing: price.voices_pricing,
            video_pricing: price.video_pricing,
            asr_pricing: price.asr_pricing,
            realtime_pricing: price.realtime_pricing,
            model_type: price.model_type,
        }
    }
}

/// `billing_tiered_prices` row.
#[derive(Debug, Clone, FromRow)]
pub struct TieredPriceRow {
    pub id: i32,
    pub model: String,
    pub region: Option<String>,
    pub currency: Option<String>,
    pub tier_type: Option<String>,
    pub tier_start: i64,
    pub tier_end: Option<i64>,
    pub input_price: i64,
    pub output_price: i64,
}

impl From<TieredPriceRow> for TieredPrice {
    fn from(row: TieredPriceRow) -> Self {
        Self {
            id: row.id,
            model: row.model,
            region: row.region,
            currency: row.currency,
            tier_type: row.tier_type,
            tier_start: row.tier_start,
            tier_end: row.tier_end,
            input_price: row.input_price,
            output_price: row.output_price,
        }
    }
}

impl From<TieredPrice> for TieredPriceRow {
    fn from(tier: TieredPrice) -> Self {
        Self {
            id: tier.id,
            model: tier.model,
            region: tier.region,
            currency: tier.currency,
            tier_type: tier.tier_type,
            tier_start: tier.tier_start,
            tier_end: tier.tier_end,
            input_price: tier.input_price,
            output_price: tier.output_price,
        }
    }
}

/// `billing_exchange_rates` row.
#[derive(Debug, Clone, FromRow)]
pub struct ExchangeRateRow {
    pub id: i32,
    pub from_currency: String,
    pub to_currency: String,
    pub rate: i64,
    pub updated_at: Option<i64>,
}

impl From<ExchangeRateRow> for ExchangeRate {
    fn from(row: ExchangeRateRow) -> Self {
        Self {
            id: row.id,
            from_currency: row.from_currency,
            to_currency: row.to_currency,
            rate: row.rate,
            updated_at: row.updated_at,
        }
    }
}

impl From<ExchangeRate> for ExchangeRateRow {
    fn from(rate: ExchangeRate) -> Self {
        Self {
            id: rate.id,
            from_currency: rate.from_currency,
            to_currency: rate.to_currency,
            rate: rate.rate,
            updated_at: rate.updated_at,
        }
    }
}
