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

// Visibility: these structs and their conversions are `pub(crate)`. They are an internal
// persistence detail -- callers of this crate see the Commerce domain types, never the rows,
// so the SQL representation cannot leak into a contract (#602 review).
use burncloud_commerce_contracts::pricing::{ExchangeRate, Price, TieredPrice};
use sqlx::FromRow;

/// `billing_prices` row.
#[derive(Debug, Clone, FromRow)]
pub(crate) struct PriceRow {
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
pub(crate) struct TieredPriceRow {
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
pub(crate) struct ExchangeRateRow {
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "unit-test module: unwrap/expect failures are the intended failure signal"
)]
mod tests {
    //! Type-level parity between the persistence rows and the Commerce domain types.
    //!
    //! These tests prove the Rust-to-Rust conversions are lossless in both directions, with every
    //! field set to a distinct value so a field added to a struct but forgotten in the conversion
    //! fails here. They deliberately do NOT touch a database.
    //!
    //! The database side -- that sqlx can actually decode these columns from SQLite, including
    //! NULL handling and the DEFAULT 0 columns -- is covered against a real database in
    //! `tests/sqlite_row_mapping.rs`. Both halves are needed: one catches a forgotten field, the
    //! other catches a column that does not decode.

    use super::*;

    /// A row with every column populated by a distinct value.
    fn full_price_row() -> PriceRow {
        PriceRow {
            id: 7,
            model: "gpt-4o".to_string(),
            currency: "USD".to_string(),
            input_price: 2_500_000_000,
            output_price: 10_000_000_000,
            cache_read_input_price: Some(250_000_000),
            cache_creation_input_price: Some(3_125_000_000),
            batch_input_price: Some(1_250_000_000),
            batch_output_price: Some(5_000_000_000),
            priority_input_price: Some(4_250_000_000),
            priority_output_price: Some(17_000_000_000),
            audio_input_price: Some(40_000_000_000),
            audio_output_price: Some(80_000_000_000),
            reasoning_price: Some(10_000_000_000),
            embedding_price: Some(20_000_000),
            image_price: Some(1_000_000),
            video_price: Some(2_000_000),
            music_price: Some(50_000_000),
            source: Some("litellm".to_string()),
            region: Some("international".to_string()),
            context_window: Some(128_000),
            max_output_tokens: Some(16_384),
            supports_vision: Some(1),
            supports_function_calling: Some(0),
            synced_at: Some(1_700_000_000),
            created_at: Some(1_600_000_000),
            updated_at: Some(1_700_000_001),
            voices_pricing: Some("{\"alloy\":15000000000}".to_string()),
            video_pricing: Some("{\"720p\":50000000}".to_string()),
            asr_pricing: Some("{\"per_minute\":360000000}".to_string()),
            realtime_pricing: Some("{\"audio_input\":32000000000}".to_string()),
            model_type: Some("chat".to_string()),
        }
    }

    #[test]
    #[allow(
        clippy::cognitive_complexity,
        reason = "test: one assertion per persisted column is clearer than splitting the parity check"
    )]
    fn price_row_round_trips_without_losing_a_column() {
        let row = full_price_row();
        let domain = Price::from(row.clone());
        let back = PriceRow::from(domain);

        assert_eq!(back.id, row.id);
        assert_eq!(back.model, row.model);
        assert_eq!(back.currency, row.currency);
        assert_eq!(back.input_price, row.input_price);
        assert_eq!(back.output_price, row.output_price);
        assert_eq!(back.cache_read_input_price, row.cache_read_input_price);
        assert_eq!(
            back.cache_creation_input_price,
            row.cache_creation_input_price
        );
        assert_eq!(back.batch_input_price, row.batch_input_price);
        assert_eq!(back.batch_output_price, row.batch_output_price);
        assert_eq!(back.priority_input_price, row.priority_input_price);
        assert_eq!(back.priority_output_price, row.priority_output_price);
        assert_eq!(back.audio_input_price, row.audio_input_price);
        assert_eq!(back.audio_output_price, row.audio_output_price);
        assert_eq!(back.reasoning_price, row.reasoning_price);
        assert_eq!(back.embedding_price, row.embedding_price);
        assert_eq!(back.image_price, row.image_price);
        assert_eq!(back.video_price, row.video_price);
        assert_eq!(back.music_price, row.music_price);
        assert_eq!(back.source, row.source);
        assert_eq!(back.region, row.region);
        assert_eq!(back.context_window, row.context_window);
        assert_eq!(back.max_output_tokens, row.max_output_tokens);
        assert_eq!(back.supports_vision, row.supports_vision);
        assert_eq!(
            back.supports_function_calling,
            row.supports_function_calling
        );
        assert_eq!(back.synced_at, row.synced_at);
        assert_eq!(back.created_at, row.created_at);
        assert_eq!(back.updated_at, row.updated_at);
        assert_eq!(back.voices_pricing, row.voices_pricing);
        assert_eq!(back.video_pricing, row.video_pricing);
        assert_eq!(back.asr_pricing, row.asr_pricing);
        assert_eq!(back.realtime_pricing, row.realtime_pricing);
        assert_eq!(back.model_type, row.model_type);
    }

    #[test]
    fn all_null_price_row_round_trips() {
        let row = PriceRow {
            id: 0,
            model: "some-model".to_string(),
            currency: "CNY".to_string(),
            input_price: 0,
            output_price: 0,
            cache_read_input_price: None,
            cache_creation_input_price: None,
            batch_input_price: None,
            batch_output_price: None,
            priority_input_price: None,
            priority_output_price: None,
            audio_input_price: None,
            audio_output_price: None,
            reasoning_price: None,
            embedding_price: None,
            image_price: None,
            video_price: None,
            music_price: None,
            source: None,
            region: None,
            context_window: None,
            max_output_tokens: None,
            supports_vision: None,
            supports_function_calling: None,
            synced_at: None,
            created_at: None,
            updated_at: None,
            voices_pricing: None,
            video_pricing: None,
            asr_pricing: None,
            realtime_pricing: None,
            model_type: None,
        };
        let back = PriceRow::from(Price::from(row));
        assert_eq!(back.cache_read_input_price, None);
        assert_eq!(back.supports_vision, None);
        assert_eq!(back.model_type, None);
        assert_eq!(back.region, None);
    }

    #[test]
    fn tiered_price_row_round_trips_without_losing_a_column() {
        let row = TieredPriceRow {
            id: 3,
            model: "qwen-max".to_string(),
            region: Some("cn".to_string()),
            currency: Some("CNY".to_string()),
            tier_type: Some("context_length".to_string()),
            tier_start: 32_000,
            tier_end: Some(128_000),
            input_price: 2_400_000_000,
            output_price: 4_800_000_000,
        };
        let back = TieredPriceRow::from(TieredPrice::from(row.clone()));

        assert_eq!(back.id, row.id);
        assert_eq!(back.model, row.model);
        assert_eq!(back.region, row.region);
        assert_eq!(back.currency, row.currency);
        assert_eq!(back.tier_type, row.tier_type);
        assert_eq!(back.tier_start, row.tier_start);
        assert_eq!(back.tier_end, row.tier_end);
        assert_eq!(back.input_price, row.input_price);
        assert_eq!(back.output_price, row.output_price);
    }

    #[test]
    fn open_ended_tier_keeps_its_missing_upper_bound() {
        let row = TieredPriceRow {
            id: 9,
            model: "qwen-max".to_string(),
            region: None,
            currency: None,
            tier_type: None,
            tier_start: 128_000,
            tier_end: None,
            input_price: 3_000_000_000,
            output_price: 6_000_000_000,
        };
        let back = TieredPriceRow::from(TieredPrice::from(row));
        assert_eq!(back.tier_end, None, "an open-ended tier has no upper bound");
        assert_eq!(back.region, None);
    }

    #[test]
    fn exchange_rate_row_round_trips_without_losing_a_column() {
        let row = ExchangeRateRow {
            id: 1,
            from_currency: "USD".to_string(),
            to_currency: "CNY".to_string(),
            rate: 7_240_000_000,
            updated_at: Some(1_700_000_000),
        };
        let back = ExchangeRateRow::from(ExchangeRate::from(row.clone()));

        assert_eq!(back.id, row.id);
        assert_eq!(back.from_currency, row.from_currency);
        assert_eq!(back.to_currency, row.to_currency);
        assert_eq!(back.rate, row.rate);
        assert_eq!(back.updated_at, row.updated_at);
        // Scaled integer rate keeps its exact precision through the conversion.
        assert_eq!(back.rate, 7_240_000_000);
    }
}
