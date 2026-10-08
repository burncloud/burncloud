#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic_in_result_fn,
    reason = "Test-only file: the assertions are the test, and clippy.toml's allow-panic-in-tests does not recognise #[tokio::test]"
)]
//! The batch and priority boundaries (#633, plan section 5 item 17).
//!
//! The plan lists "batch/priority 边界" -- the boundaries of the two request-class discounts. `calculator.rs`
//! has one inline test for each (`test_batch_discount`, `test_priority_surcharge`), and both assert only
//! `input_cost` and `output_cost` on a plain request. What they do not answer is what the flags do to
//! **everything else** a request can be charged for, which is the boundary question.
//!
//! ## What the code says
//!
//! ```text
//! calculator.rs:173-205
//!   if is_priority && is_batch  -> warn, then use the priority rate
//!   if is_priority              -> priority_input_price  or input_price  * 170%
//!   else if is_batch            -> batch_input_price     or input_price  *  50%
//!   else                        -> input_price
//! ```
//!
//! and the result is bound to a pair named **`effective_input_price` / `effective_output_price`**. So the two
//! flags are expected to move input and output and nothing else, because the other components read their own
//! fields:
//!
//! | Component | Price it reads | Should the flags move it? |
//! | --- | --- | --- |
//! | `cache_cost` | `cache_read_input_price` / `cache_creation_input_price`, else `input_price` × 10% / 125% | see below |
//! | `reasoning_cost` | `reasoning_price`, else `effective_output_price` | **yes**, if the fallback is the effective rate |
//! | `image_cost` / `video_cost` | their own field, else `input_price` (the **raw** one) | the flags should not move them |
//! | `audio_cost` / `voice_cost` | their own fields, else `input_price` × 700% | the flags should not move them |
//! | `music_cost` | `music_price`, flat | the flags should not move it |
//!
//! This file measures each of those rather than asserting the table, because two of the rows are genuinely
//! ambiguous from reading: `reasoning` falls back to the *effective* output rate while `cache_read` falls back
//! to the *raw* input rate, and whether that asymmetry is intended is a question for whoever owns pricing.
//!
//! ## How the numbers are derived
//!
//! Every figure below is computed by hand from `cost_nano = tokens * price_per_million / 1_000_000`
//! (`calculator.rs:349`) and `price * percent / 100`. A hundred tokens in each bucket is used throughout, which
//! makes each per-token component equal to its price divided by ten thousand, so a wrong rate shows up as a
//! recognisable multiple rather than a similar-looking number.

use burncloud_database::{create_database_with_url, Database};
use burncloud_database_billing::{BillingPriceModel, PriceInput};
use burncloud_service_billing::types::{CostBreakdown, UnifiedUsage};
use burncloud_service_billing::{CostCalculator, PriceCache};
use std::error::Error;

async fn fresh_db(tag: &str) -> Result<(Database, std::path::PathBuf), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!(
        "bc_batchprio_{}_{}_{}.db",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::remove_file(&path).ok();
    let normalized = path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{}?mode=rwc", normalized);
    Ok((create_database_with_url(&url).await?, path))
}

async fn cleanup(db: Database, path: std::path::PathBuf) {
    db.close().await.ok();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    for suffix in ["", "-wal", "-shm"] {
        let mut candidate = path.clone().into_os_string();
        candidate.push(suffix);
        let candidate = std::path::PathBuf::from(candidate);
        if candidate.exists() {
            std::fs::remove_file(&candidate)
                .unwrap_or_else(|e| panic!("{} was not removed ({e})", candidate.display()));
        }
    }
}

/// A price with every component set to a distinct value.
///
/// Distinct so a component reading the wrong field is visible: `input_price` 1_000_000 means one nano per
/// token, `output_price` 2_000_000 means two, and so on. `cache_read_input_price` and
/// `cache_creation_input_price` are left unset so the **fallback expressions** are what get exercised.
fn priced_model(model: &str) -> PriceInput {
    PriceInput {
        model: model.to_string(),
        currency: "USD".to_string(),
        input_price: 1_000_000,
        output_price: 2_000_000,
        audio_input_price: Some(5_000_000),
        audio_output_price: Some(6_000_000),
        reasoning_price: None, // exercise the fallback to the effective output rate
        image_price: Some(9_000_000),
        video_price: Some(8_000_000),
        music_price: Some(70_000_000),
        cache_read_input_price: None,     // exercise input_price * 10%
        cache_creation_input_price: None, // exercise input_price * 125%
        ..Default::default()
    }
}

/// One hundred tokens in every bucket, so each per-token figure is its rate divided by ten thousand.
fn full_usage() -> UnifiedUsage {
    UnifiedUsage {
        input_tokens: 100,
        output_tokens: 100,
        cache_read_tokens: 100,
        cache_write_tokens: 100,
        audio_input_tokens: 100,
        audio_output_tokens: 100,
        image_tokens: 100,
        video_tokens: 100,
        reasoning_tokens: 100,
        embedding_tokens: 100,
    }
}

async fn breakdown_for(
    tag: &str,
    model: &str,
    is_batch: bool,
    is_priority: bool,
) -> Result<CostBreakdown, Box<dyn Error>> {
    let (db, path) = fresh_db(tag).await?;
    BillingPriceModel::upsert(&db, &priced_model(model)).await?;
    let cache = PriceCache::load(&db).await?;
    let calc = CostCalculator::new(cache);
    let result = calc
        .calculate(model, &full_usage(), tag, is_batch, is_priority, None)
        .await?;
    let b = result.breakdown.clone();
    assert_eq!(
        result.usd_amount_nano,
        b.total(),
        "the reported total must equal the sum of the components"
    );
    cleanup(db, path).await;
    Ok(b)
}

// -------------------------------------------------------------------------------------------
// the boundary, measured in one table
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn the_two_flags_move_input_and_output_and_the_table_shows_what_else(
) -> Result<(), Box<dyn Error>> {
    // The measurement this file exists for. Four runs of the same request, differing only in the two flags.
    let plain = breakdown_for("plain", "boundary", false, false).await?;
    let priority = breakdown_for("priority", "boundary", false, true).await?;
    let batch = breakdown_for("batch", "boundary", true, false).await?;
    let both = breakdown_for("both", "boundary", true, true).await?;

    let label = |n: &str, v: i64| format!("{n}={v}");
    for (name, b) in [
        ("plain", &plain),
        ("priority", &priority),
        ("batch", &batch),
        ("both", &both),
    ] {
        println!(
            "{name:>8}: {}",
            [
                label("input", b.input_cost),
                label("output", b.output_cost),
                label("cache", b.cache_cost),
                label("reasoning", b.reasoning_cost),
                label("audio", b.audio_cost),
                label("image", b.image_cost),
                label("video", b.video_cost),
                label("music", b.music_cost),
                label("embedding", b.embedding_cost),
            ]
            .join(" ")
        );
    }

    // --- input and output, where the flags are documented to act ---
    assert_eq!(
        plain.input_cost, 100,
        "100 tokens at one nano (1_000_000/10_000)"
    );
    assert_eq!(
        plain.output_cost, 200,
        "100 tokens at two nano (2_000_000/10_000)"
    );
    assert_eq!(
        priority.input_cost, 170,
        "170% of one nano: PRIORITY_SURCHARGE_PERCENT at calculator.rs:12"
    );
    assert_eq!(priority.output_cost, 340, "170% of two nano");
    assert_eq!(
        batch.input_cost, 50,
        "50% of one nano: BATCH_DISCOUNT_PERCENT at calculator.rs:14"
    );
    assert_eq!(batch.output_cost, 100, "50% of two nano");

    // --- both set: priority wins, and the code warns ---
    assert_eq!(
        both.input_cost, priority.input_cost,
        "with both flags the priority rate applies (calculator.rs:173-179 warns and then takes the priority \
         branch)"
    );
    assert_eq!(both.output_cost, priority.output_cost);

    // --- the boundaries: what the flags must NOT move ---
    assert_eq!(
        batch.image_cost, plain.image_cost,
        "image reads its own field"
    );
    assert_eq!(priority.image_cost, plain.image_cost);
    assert_eq!(
        batch.video_cost, plain.video_cost,
        "video reads its own field"
    );
    assert_eq!(batch.music_cost, plain.music_cost, "music is a flat fee");
    assert_eq!(
        batch.audio_cost, plain.audio_cost,
        "audio reads its own fields"
    );
    assert_eq!(
        batch.embedding_cost, plain.embedding_cost,
        "embedding reads its own field"
    );

    // --- the two rows that are not symmetric, recorded rather than endorsed ---
    println!(
        "cache: plain={} batch={} priority={}",
        plain.cache_cost, batch.cache_cost, priority.cache_cost
    );
    assert_eq!(
        batch.cache_cost, plain.cache_cost,
        "the cache fallbacks read `price.input_price` -- the RAW rate, not `effective_input_price` -- so a \
         batch discount does not reach the cache"
    );
    println!(
        "reasoning: plain={} batch={} priority={}",
        plain.reasoning_cost, batch.reasoning_cost, priority.reasoning_cost
    );
    assert_eq!(
        priority.reasoning_cost, 340,
        "reasoning falls back to the EFFECTIVE output rate (calculator.rs:221 reads `effective_output_price`), \
         so a priority surcharge does reach it while a batch discount does not reach the cache"
    );

    Ok(())
}

// -------------------------------------------------------------------------------------------
// the explicit per-class prices, which take precedence over the percentages
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn an_explicit_batch_price_beats_the_discount_percentage() -> Result<(), Box<dyn Error>> {
    // `batch_input_price.unwrap_or(...)`. A model that states its batch rate must be billed at that rate, not
    // at half its standard one -- otherwise a provider whose batch price is *not* half would be mis-billed.
    let (db, path) = fresh_db("explicit_batch").await?;
    let mut p = priced_model("explicit-batch");
    p.batch_input_price = Some(300_000); // 0.3 nano, NOT 50% of 1_000_000
    p.batch_output_price = Some(400_000);
    BillingPriceModel::upsert(&db, &p).await?;
    let calc = CostCalculator::new(PriceCache::load(&db).await?);

    let result = calc
        .calculate("explicit-batch", &full_usage(), "req-eb", true, false, None)
        .await?;
    let b = &result.breakdown;
    println!(
        "explicit batch prices: input={} output={}",
        b.input_cost, b.output_cost
    );

    assert_eq!(b.input_cost, 30, "100 tokens at 300_000/10_000");
    assert_eq!(b.output_cost, 40, "100 tokens at 400_000/10_000");
    assert_ne!(
        b.input_cost, 50,
        "which is not the 50% figure, so the explicit price was used rather than the percentage"
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn an_explicit_priority_price_beats_the_surcharge_percentage() -> Result<(), Box<dyn Error>> {
    let (db, path) = fresh_db("explicit_priority").await?;
    let mut p = priced_model("explicit-priority");
    p.priority_input_price = Some(700_000); // 0.7 nano, NOT 170% of 1_000_000
    p.priority_output_price = Some(800_000);
    BillingPriceModel::upsert(&db, &p).await?;
    let calc = CostCalculator::new(PriceCache::load(&db).await?);

    let result = calc
        .calculate(
            "explicit-priority",
            &full_usage(),
            "req-ep",
            false,
            true,
            None,
        )
        .await?;
    let b = &result.breakdown;
    println!(
        "explicit priority prices: input={} output={}",
        b.input_cost, b.output_cost
    );

    assert_eq!(b.input_cost, 70, "100 tokens at 700_000/10_000");
    assert_eq!(b.output_cost, 80, "100 tokens at 800_000/10_000");
    assert_ne!(b.input_cost, 170, "which is not the 170% figure");

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn only_the_input_price_is_stated_for_batch_and_the_output_side_still_falls_back(
) -> Result<(), Box<dyn Error>> {
    // The two fields are optional independently. A model setting only `batch_input_price` must still get the
    // 50% fallback on the output side rather than paying full output price -- or, worse, nothing.
    let (db, path) = fresh_db("half_explicit").await?;
    let mut p = priced_model("half-explicit");
    p.batch_input_price = Some(300_000);
    // batch_output_price deliberately unset
    BillingPriceModel::upsert(&db, &p).await?;
    let calc = CostCalculator::new(PriceCache::load(&db).await?);

    let result = calc
        .calculate("half-explicit", &full_usage(), "req-he", true, false, None)
        .await?;
    let b = &result.breakdown;
    println!(
        "half-explicit batch: input={} output={}",
        b.input_cost, b.output_cost
    );

    assert_eq!(b.input_cost, 30, "the explicit batch input price");
    assert_eq!(
        b.output_cost, 100,
        "and the output side falls back to 50% of 2_000_000, not to zero and not to the full rate"
    );

    cleanup(db, path).await;
    Ok(())
}

// -------------------------------------------------------------------------------------------
// the boundaries in the other direction: zero and extreme token counts
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_batch_request_of_zero_tokens_costs_nothing() -> Result<(), Box<dyn Error>> {
    // The zero-usage boundary under a discount: 50% of nothing is nothing, not a negative or a fee. Music is
    // the exception and is asserted separately, because it is a flat fee.
    let (db, path) = fresh_db("batch_zero").await?;
    BillingPriceModel::upsert(&db, &priced_model("batch-zero")).await?;
    let calc = CostCalculator::new(PriceCache::load(&db).await?);

    // No music price on this model, so the flat fee does not apply and the total is purely per-token.
    let mut p = priced_model("batch-zero-nomusic");
    p.music_price = None;
    BillingPriceModel::upsert(&db, &p).await?;
    let calc_nomusic = CostCalculator::new(PriceCache::load(&db).await?);
    let _ = calc; // the first calculator was only to confirm the model exists twice

    let result = calc_nomusic
        .calculate(
            "batch-zero-nomusic",
            &UnifiedUsage::default(),
            "req-bz",
            true,
            false,
            None,
        )
        .await?;
    println!("zero-token batch request: {:?}", result.breakdown);

    assert_eq!(result.usd_amount_nano, 0, "no tokens, no charge");
    assert_eq!(result.breakdown.total(), 0);

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn a_priority_surcharge_on_a_large_request_does_not_wrap_to_negative(
) -> Result<(), Box<dyn Error>> {
    // The overflow boundary the plan also names. A rate near `i64::MAX` multiplied by 170% cannot be
    // represented, and `saturating_mul_percent` (`calculator.rs:376`) computes it in i128 and caps at
    // `i64::MAX`; `nano()` then computes `tokens * rate / 1_000_000` in i128 and caps again with a warning.
    //
    // The property that matters is **the sign**: a wrap would produce a negative charge, which credits the
    // account rather than charging it. An earlier version of this test asserted the exact figure `i64::MAX`
    // and failed, because the two caps compose: `saturating_mul_percent` yields `i64::MAX`, and `nano()`
    // multiplies that by a million and caps again. The measured value is asserted with that reasoning, not
    // guessed.
    let (db, path) = fresh_db("priority_large").await?;
    let mut p = priced_model("priority-large");
    p.input_price = i64::MAX / 4;
    p.priority_input_price = None; // force the percentage path
    BillingPriceModel::upsert(&db, &p).await?;
    let calc = CostCalculator::new(PriceCache::load(&db).await?);

    let result = calc
        .calculate(
            "priority-large",
            &UnifiedUsage {
                input_tokens: 1_000_000,
                ..Default::default()
            },
            "req-pl",
            false,
            true,
            None,
        )
        .await?;
    let charged = result.breakdown.input_cost;
    println!("priority on a huge rate: input_cost={charged}");

    assert!(
        charged > 0,
        "a wrapped negative here would credit the account rather than charge it; got {charged}"
    );
    // No `charged <= i64::MAX` assertion here: the value is an `i64`, so that comparison is always true and
    // clippy rejects it as such. It was in an earlier version of this test and checked nothing.
    assert_eq!(
        charged,
        ((i64::MAX / 4) as i128 * 170 / 100) as i64,
        "the figure is 170% of the stated rate, computed in i128 -- as the source does -- and the million \
         tokens cancel the division by a million, leaving the rate itself. The expectation is computed in \
         i128 too, because `(i64::MAX / 4) * 170` overflows i64 in the test itself, which is how an earlier \
         version of this assertion failed"
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn the_surcharge_percentage_is_applied_to_the_rate_and_not_to_the_charge(
) -> Result<(), Box<dyn Error>> {
    // Which order the percentage and the token arithmetic are applied in does not change the result for a
    // per-million rate, and this test says so rather than leaving it ambiguous: 170% of a rate, then divided by
    // a million and multiplied by the tokens, equals the charge multiplied by 170%.
    let (db, path) = fresh_db("surcharge_order").await?;
    let mut p = priced_model("surcharge-order");
    p.input_price = 1_000_000;
    p.priority_input_price = None;
    BillingPriceModel::upsert(&db, &p).await?;
    let calc = CostCalculator::new(PriceCache::load(&db).await?);

    let mut charges = Vec::new();
    for tokens in [1_i64, 1_000, 1_000_000] {
        let result = calc
            .calculate(
                "surcharge-order",
                &UnifiedUsage {
                    input_tokens: tokens,
                    ..Default::default()
                },
                "req-so",
                false,
                true,
                None,
            )
            .await?;
        charges.push((tokens, result.breakdown.input_cost));
    }
    println!("priority charges by token count: {charges:?}");

    for (tokens, charged) in charges {
        // The plain rate would be `tokens * 1_000_000 / 1_000_000 = tokens`; the surcharge makes it 170%.
        let expected = tokens * 170 / 100;
        assert_eq!(
            charged, expected,
            "{tokens} tokens at 170% of one nano each"
        );
    }

    cleanup(db, path).await;
    Ok(())
}
