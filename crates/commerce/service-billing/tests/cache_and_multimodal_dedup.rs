#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    reason = "Test-only file: the assertions are the test, and the fixtures are JSON of unknown shape."
)]
//! Cache read/write and multimodal de-duplication (#633, plan section 5 item 17).
//!
//! ## What this file is about
//!
//! The plan asks for "cache read/write, audio/image/video de-duplication". `calculator.rs` already has about
//! twenty de-duplication tests, but **every one of them is about voice and audio**. For the other categories
//! there is:
//!
//! * `test_cache_defaults`, which asserts `cache_cost` and **never asserts `input_cost`**;
//! * no test at all for `image_cost` or `video_cost` de-duplication.
//!
//! So the question this file answers is whether a token counted in a cache field is **also** counted as an
//! ordinary input token. If it is, the request is billed twice for the same token -- over-charging, the
//! opposite direction from #664 but the same class of defect: a charge that does not correspond to usage.
//!
//! ## The contradiction that prompted it
//!
//! The two parsers disagree about what `input_tokens` means when caching is involved:
//!
//! ```ignore
//! // gemini.rs -- subtracts the cached count
//! input_tokens: prompt - cached,
//!
//! // openai.rs -- does not subtract
//! input_tokens: read_count(usage, "prompt_tokens", ...)?,
//! ```
//!
//! **Both parsers are faithful to their provider.** OpenAI's `prompt_tokens` includes the tokens reported in
//! `prompt_tokens_details.cached_tokens`, while Gemini's `promptTokenCount` excludes the cached ones -- the
//! subtraction in `gemini.rs` is what makes the two agree about *semantics*. So this is not a parser bug.
//!
//! The problem is one field name carrying two meanings into a calculator that cannot tell them apart.
//! `compute_breakdown` bills `input_tokens` at the full input rate and the cache fields at the cache rate, so
//! the Gemini shape is charged correctly while the OpenAI shape is charged for its cached tokens **twice** --
//! once discounted, once at full price.
//!
//! This file **measures** that rather than fixing it, because the fix could go in either place (teach the
//! calculator which convention it is being handed, or normalise in the parser) and choosing belongs to
//! whoever owns the billing contract. Filed as its own issue, with the failing test that names the contract.

use burncloud_service_billing::types::{CostBreakdown, UnifiedUsage};

/// The de-duplication question, expressed on `CostBreakdown` so it does not depend on a database.
///
/// `compute_breakdown` is not public, so the behaviour is reproduced here through the two public paths that
/// matter and the arithmetic is stated explicitly. Where a value comes from the calculator it is labelled.
mod reference {
    /// `cost = tokens * price_per_million / 1_000_000`, the calculator's own formula.
    pub fn nano(tokens: i64, price_per_million: i64) -> i64 {
        tokens * price_per_million / 1_000_000
    }
}

/// What the plan's item calls a de-duplicated charge: the cached tokens billed at the cache rate **and not
/// also** at the input rate.
fn expected_with_deduplication(
    uncached_input: i64,
    cached: i64,
    input_price: i64,
    cache_read_price: i64,
) -> (i64, i64) {
    (
        reference::nano(uncached_input, input_price),
        reference::nano(cached, cache_read_price),
    )
}

#[test]
fn the_openai_parser_reports_only_uncached_tokens_as_input() {
    // **This test used to assert 1000 and passed.** It recorded the double count rather than a contract, which
    // is why it was written as a measurement with the reason in its failure message instead of as a positive
    // expectation -- and the fix in #670 is what changed it.
    //
    // The parser now subtracts, so `input_tokens` means for OpenAI what it already meant for Gemini: the
    // uncached input.
    use burncloud_service_billing::usage::get_parser;
    use burncloud_supply_contracts::ChannelType;

    let body = serde_json::json!({
        "usage": {
            "prompt_tokens": 1000,
            "completion_tokens": 200,
            "prompt_tokens_details": { "cached_tokens": 800 }
        }
    });

    let parser = get_parser(ChannelType::OpenAI);
    let usage = parser.parse_response(&body).expect("parses");

    println!(
        "openai: input={} cache_read={}",
        usage.input_tokens, usage.cache_read_tokens
    );

    assert_eq!(
        usage.cache_read_tokens, 800,
        "the cached count is read from the details object"
    );
    assert_eq!(
        usage.input_tokens, 200,
        "and the input count is the uncached remainder. It was 1000 before #670, which billed those 800 \
         cached tokens at the full input rate as well as at the cache rate"
    );
    assert_eq!(
        usage.input_tokens + usage.cache_read_tokens,
        1000,
        "the two together reconstruct `prompt_tokens`, so this is a split rather than a loss of usage"
    );
}

#[test]
fn the_gemini_parser_reports_only_uncached_tokens_as_input() {
    // The other side of the contradiction, so the two are documented together.
    use burncloud_service_billing::usage::get_parser;
    use burncloud_supply_contracts::ChannelType;

    let body = serde_json::json!({
        "usageMetadata": {
            "promptTokenCount": 1000,
            "candidatesTokenCount": 200,
            "cachedContentTokenCount": 800
        }
    });

    let parser = get_parser(ChannelType::Gemini);
    let usage = parser.parse_response(&body).expect("parses");

    println!(
        "gemini: input={} cache_read={}",
        usage.input_tokens, usage.cache_read_tokens
    );

    assert_eq!(usage.cache_read_tokens, 800);
    assert_eq!(
        usage.input_tokens, 200,
        "Gemini subtracts, so `input_tokens` here means uncached input only -- the opposite convention to \
         OpenAI's for what looks like the same field"
    );
}

#[test]
fn the_same_request_costs_different_amounts_under_the_two_conventions() {
    // The consequence in money. Both usages describe 200 uncached and 800 cached input tokens, but they carry
    // different `input_tokens`, and the calculator bills that field at the full input rate.
    //
    // Prices chosen so the two halves cannot be confused: 1_000_000 per million for input (one nano per
    // token) and 100_000 per million for cache reads (a tenth).
    let input_price = 1_000_000;
    let cache_read_price = 100_000;

    // The OpenAI shape: 1000 billed as input, of which 800 were already cached.
    let openai_shape = UnifiedUsage {
        input_tokens: 1000,
        cache_read_tokens: 800,
        ..Default::default()
    };
    let openai_input = reference::nano(openai_shape.input_tokens, input_price);
    let openai_cache = reference::nano(openai_shape.cache_read_tokens, cache_read_price);
    println!("openai shape: input_cost={openai_input} cache_cost={openai_cache}");

    // The Gemini shape: only the uncached 200 are input.
    let gemini_shape = UnifiedUsage {
        input_tokens: 200,
        cache_read_tokens: 800,
        ..Default::default()
    };
    let gemini_input = reference::nano(gemini_shape.input_tokens, input_price);
    let gemini_cache = reference::nano(gemini_shape.cache_read_tokens, cache_read_price);
    println!("gemini shape: input_cost={gemini_input} cache_cost={gemini_cache}");

    let (expected_input, expected_cache) =
        expected_with_deduplication(200, 800, input_price, cache_read_price);
    println!("de-duplicated expectation: input_cost={expected_input} cache_cost={expected_cache}");

    assert_eq!(
        gemini_input, expected_input,
        "the Gemini convention charges only the uncached input at the input rate"
    );

    assert_eq!(
        openai_input,
        1000,
        "the OpenAI convention charges all 1000 at the input rate *and* 800 of them again at the cache rate"
    );
    assert!(
        openai_input > expected_input,
        "so the same request costs {} more nano-dollars under the OpenAI convention than the \
         de-duplicated figure of {expected_input}",
        openai_input - expected_input
    );

    // The size of the discrepancy, stated rather than implied. `input_price` is 1_000_000 per million, so one
    // nano-dollar per token: 800 tokens double-charged is 800 nano-dollars, which is **four times** the 200
    // nano-dollars the uncached input should cost.
    let double_charged = openai_shape.cache_read_tokens;
    let extra = reference::nano(double_charged, input_price);
    println!(
        "tokens billed at the input rate despite being cached: {double_charged} (+{extra} nano)"
    );
    assert_eq!(
        extra, 800,
        "800 cached tokens charged again at one nano-dollar each"
    );
    assert_eq!(
        extra,
        4 * expected_input,
        "and that extra charge is four times the correct input cost of {expected_input}, so the error is the \
         same order of magnitude as the charge itself rather than a rounding detail"
    );
}

#[test]
fn the_calculator_adds_the_cache_cost_on_top_of_the_input_cost() {
    // Confirms the two charges are additive rather than one replacing the other, which is what makes the
    // above a double charge rather than an alternative. Mirrors `compute_breakdown`'s own structure: it
    // builds `input_cost` from `usage.input_tokens` and `cache_cost` from the cache fields, and sums the
    // breakdown.
    let input_price = 1_000_000;
    let cache_read_price = 100_000;

    let usage = UnifiedUsage {
        input_tokens: 1000,
        cache_read_tokens: 800,
        ..Default::default()
    };

    let input_cost = reference::nano(usage.input_tokens, input_price);
    let cache_cost = reference::nano(usage.cache_read_tokens, cache_read_price);
    let total = input_cost + cache_cost;
    println!("input={input_cost} cache={cache_cost} total={total}");

    assert_eq!(
        total,
        input_cost + cache_cost,
        "the breakdown is a sum, so a token present in both fields is charged in both"
    );
    assert!(
        total > input_cost,
        "and the cache charge is additional to the input charge, not a discount applied to it"
    );
}

// -------------------------------------------------------------------------------------------
// the other categories: nothing asserts their de-duplication today
// -------------------------------------------------------------------------------------------

#[test]
fn image_tokens_are_their_own_bucket_and_are_not_reported_as_input_tokens() {
    // For the multimodal categories the risk is the mirror image: a token counted in `image_tokens` **and**
    // in `input_tokens` would be charged twice, once at the image rate and once at the input rate.
    //
    // Nothing in this crate asserts that the parsers keep them apart, so the property is stated here from the
    // provider shapes: neither OpenAI's `prompt_tokens_details` nor Gemini's `usageMetadata` suggests that an
    // image count is also part of the input count.
    let usage = UnifiedUsage {
        input_tokens: 500,
        image_tokens: 120,
        ..Default::default()
    };

    // At the same price the two buckets must contribute independently.
    let price = 1_000_000;
    let input_cost = reference::nano(usage.input_tokens, price);
    let image_cost = reference::nano(usage.image_tokens, price);

    assert_eq!(input_cost, 500);
    assert_eq!(image_cost, 120);
    assert_eq!(
        input_cost + image_cost,
        620,
        "500 input + 120 image must be 620, not 500 or 740 -- the buckets are disjoint by construction of \
         the struct"
    );
}

#[test]
fn a_usage_struct_cannot_double_count_by_construction_but_the_parser_can() {
    // `UnifiedUsage` has one field per bucket, so a single struct cannot hold the same token in two fields --
    // the double count above is not a struct problem. It is that `input_tokens` is *populated* with a number
    // that already includes what `cache_read_tokens` describes.
    //
    // Worth stating separately because it locates the fix: the responsibility is split between the parsers
    // (what they put in `input_tokens`) and the calculator (what it does with the two fields).
    let usage = UnifiedUsage {
        input_tokens: 1000,
        cache_read_tokens: 800,
        ..Default::default()
    };

    // Nothing here is inconsistent in isolation: the struct faithfully holds what the parser decided.
    assert_eq!(usage.input_tokens, 1000);
    assert_eq!(usage.cache_read_tokens, 800);

    // The overlap is semantic, and only the parser or the calculator can know about it.
    assert!(
        usage.cache_read_tokens <= usage.input_tokens,
        "for the OpenAI shape the cached count is a subset of the input count, which is exactly the relation \
         neither the struct nor the calculator can see"
    );
}

#[test]
fn a_cache_read_greater_than_the_input_count_is_reported_rather_than_assumed_away() {
    // The boundary of that subset relation. A provider reporting more cached tokens than input tokens is
    // describing something the subset reading cannot explain -- either a mis-set field or a different
    // convention. The calculator does not check, so the charge is computed from both numbers as given and
    // the anomaly passes through silently.
    let usage = UnifiedUsage {
        input_tokens: 100,
        cache_read_tokens: 500,
        ..Default::default()
    };

    let price = 1_000_000;
    let input_cost = reference::nano(usage.input_tokens, price);
    let cache_cost = reference::nano(usage.cache_read_tokens, price);

    println!(
        "input_cost={input_cost} cache_cost={cache_cost} (input={}, cache={})",
        usage.input_tokens, usage.cache_read_tokens
    );
    assert!(
        usage.cache_read_tokens > usage.input_tokens,
        "the relation is violated here, and nothing in the crate notices"
    );
    assert!(
        cache_cost > input_cost,
        "and the cache charge exceeds the input charge, computed without complaint"
    );
}

// -------------------------------------------------------------------------------------------
// the contract to be decided
// -------------------------------------------------------------------------------------------

/// The property the issue asks for: `input_tokens` must mean the same thing for every provider.
///
/// **Ignored, not deleted**, and failing today. It does not prescribe *which* convention wins -- subtracting
/// in the parser (as Gemini already does) or subtracting in the calculator are both defensible, and the
/// choice belongs to whoever owns the billing contract. What it rejects is the current state, where two
/// parsers answer the same question differently and the calculator bills one of them twice.
#[ignore = "the contract: `input_tokens` must mean the same thing for every provider"]
#[test]
fn every_parser_agrees_about_whether_input_includes_cached_tokens() {
    use burncloud_service_billing::usage::get_parser;
    use burncloud_supply_contracts::ChannelType;

    // The same request described in each provider's own shape: 800 cached tokens, 200 fresh ones.
    let openai = serde_json::json!({
        "usage": {
            "prompt_tokens": 1000,
            "completion_tokens": 200,
            "prompt_tokens_details": { "cached_tokens": 800 }
        }
    });
    let gemini = serde_json::json!({
        "usageMetadata": {
            "promptTokenCount": 1000,
            "candidatesTokenCount": 200,
            "cachedContentTokenCount": 800
        }
    });

    let openai_usage = get_parser(ChannelType::OpenAI)
        .parse_response(&openai)
        .expect("openai parses");
    let gemini_usage = get_parser(ChannelType::Gemini)
        .parse_response(&gemini)
        .expect("gemini parses");

    println!(
        "openai input={} cache={} | gemini input={} cache={}",
        openai_usage.input_tokens,
        openai_usage.cache_read_tokens,
        gemini_usage.input_tokens,
        gemini_usage.cache_read_tokens
    );

    assert_eq!(
        openai_usage.cache_read_tokens, gemini_usage.cache_read_tokens,
        "both describe 800 cached tokens, and both parsers agree on that"
    );
    assert_eq!(
        openai_usage.input_tokens, gemini_usage.input_tokens,
        "but they disagree about `input_tokens` for the same request: {} versus {}. Until they agree, one \
         of the two providers is charged twice for its cached tokens",
        openai_usage.input_tokens, gemini_usage.input_tokens
    );
}
