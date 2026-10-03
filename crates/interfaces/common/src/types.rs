// S1-F: this module now holds only re-exports. Every type that used to be declared here has been
// moved to its owning domain contract, deleted as a duplicate, or retired as dead code:
//   * prices and pricing     -> burncloud_commerce_contracts (S1-A, S1-B)
//   * channel, ability       -> burncloud_supply_contracts   (S1-C)
//   * User, Token, Recharge  -> deleted in S1-D (no consumer)
//   * OpenAI DTOs, colour    -> burncloud_traffic_contracts  (S1-E)
//   * ProtocolConfig         -> deleted in S1-F: field-for-field duplicate of Supply's
//                               ChannelProtocolConfig, with no consumer
// The module stays so that `burncloud_common::types::*` keeps working for existing consumers.

// Re-export nanodollar conversion utilities owned by Commerce, keeping the legacy
// alias names used by existing consumers of this module.
pub use burncloud_commerce_contracts::price_u64::{
    dollars_to_nano as dollars_to_nanodollars, nano_to_dollars as nanodollars_to_dollars,
    NANO_PER_DOLLAR as NANODOLLAR_SCALE,
};

// ---------------------------------------------------------------------------
// S1-B: the price and pricing value objects moved to the Commerce contract crate
// (`burncloud_commerce_contracts::pricing`). They are re-exported here under the same
// names and module path, so existing `burncloud_common::types::*` consumers keep
// compiling. `ModelMetadata` and the `pricing_config` document schema stay in
// `burncloud_common::pricing_config`.
// ---------------------------------------------------------------------------
pub use burncloud_commerce_contracts::pricing::{
    BillingExchangeRate, BillingPrice, BillingPriceInput, BillingTieredPrice,
    BillingTieredPriceInput, Currency, ExchangeRate, FullPricing, MultiCurrencyPrice, Price,
    PriceInput, TieredPrice, TieredPriceInput,
};

// ---------------------------------------------------------------------------
// S1-E: the Traffic-owned protocol DTOs and the scheduling colour moved to the
// Traffic contract crate (`burncloud_traffic_contracts`). They are re-exported here under the
// same names and module path, so existing `burncloud_common::types::*` consumers keep
// compiling. `RequestMapping`/`ResponseMapping` were NOT moved: the copies that used to live
// here had no consumer and duplicated `traffic/router/src/adaptor/mapping.rs`, so they were
// deleted instead.
// ---------------------------------------------------------------------------
pub use burncloud_traffic_contracts::{
    OpenAIChatChoice, OpenAIChatMessage, OpenAIChatRequest, OpenAIChatResponse, TrafficColor,
};

// ---------------------------------------------------------------------------
// S1-C: the Supply-owned channel and ability types moved to the Supply contract crate
// (`burncloud_supply_contracts`). They are re-exported here under the same names and module path
// so existing `burncloud_common::types::*` and root-level consumers keep compiling.
// S1-D removed the legacy `Recharge`/`User`/`Token` DTOs that used to live here: they had no
// consumer anywhere in the workspace and duplicated the Identity types.
// ---------------------------------------------------------------------------
pub use burncloud_supply_contracts::{
    Ability, Channel, ChannelAbility, ChannelProvider, ChannelType,
};
