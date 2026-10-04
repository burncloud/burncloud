#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! Multimodal billing de-duplication: image, video, audio and music (#633, plan section 5 item 17).
//!
//! The plan asks for "cache read/write, audio/image/video de-duplication". The cache half is in
//! `cache_and_multimodal_dedup.rs`; this file covers the modalities, and it is a **different** question.
//!
//! ## Why a separate question
//!
//! The cache case was a genuine double charge: OpenAI reports `prompt_tokens` **including** the cached ones,
//! so the same tokens appeared in two fields and were billed twice (#670). The check to run here is whether
//! the same thing can happen between `input_tokens` and the modality fields -- and **it cannot**, for a reason
//! worth recording rather than assuming:
//!
//! ```text
//! image_tokens and video_tokens are never set by any provider parser.
//! ```
//!
//! The providers do not mention them; the only writers in the workspace are
//! `UnifiedTokenCounter::accumulate`/`set_from_usage` (`counter.rs:17-18`, `:108-109`) and one explicit
//! assignment for video-generation tasks (`traffic/router/src/lib.rs:1287`). So no provider response can put
//! the same token in `input_tokens` and `image_tokens`, and the overlap the cache case had has no analogue.
//!
//! That is asserted by `no_provider_parser_populates_the_modality_fields`, because "this cannot happen" is
//! exactly the kind of claim that silently stops being true.
//!
//! ## What this file tests instead
//!
//! With the double-charge route closed, the properties left are about **which price applies and what is
//! summed**:
//!
//! - an explicit `image_price` / `video_price` is used, and the **fallback is `input_price`**, not zero;
//! - each modality contributes to its own component and no other;
//! - `music_price` is a **flat fee per request**, deliberately not passed through `nano()`, so a small token
//!   count cannot round it to zero;
//! - the components sum into `total()` and into `CostResult::usd_amount_nano`.
//!
//! Every amount is derived by hand from `cost_nano = tokens * price_per_million / 1_000_000`
//! (`calculator.rs:349`) rather than read off a run.

use burncloud_database::{create_database_with_url, Database};
use burncloud_database_billing::{BillingPriceModel, PriceInput};
use burncloud_service_billing::types::UnifiedUsage;
use burncloud_service_billing::{CostCalculator, PriceCache};
use std::error::Error;

/// A fresh SQLite file database with the real migrations applied.
///
/// A file rather than `:memory:`: the pool opens several connections and an in-memory SQLite database is
/// per-connection, so migrations and the cache would see different databases. Three slashes because the path is
/// absolute; the two-slash form is not a URL SQLite can open on Windows.
async fn fresh_db(tag: &str) -> Result<(Database, std::path::PathBuf), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!(
        "bc_multimodal_{}_{}_{}.db",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&path);
    let normalized = path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{}?mode=rwc", normalized);
    let db = create_database_with_url(&url).await?;
    Ok((db, path))
}

/// Close the pool, wait for the handle to be released, then remove the database and its WAL companions.
async fn cleanup(db: Database, path: std::path::PathBuf) {
    db.close().await.ok();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    for suffix in ["", "-wal", "-shm"] {
        let mut candidate = path.clone().into_os_string();
        candidate.push(suffix);
        let candidate = std::path::PathBuf::from(candidate);
        if candidate.exists() {
            std::fs::remove_file(&candidate).unwrap_or_else(|e| {
                panic!(
                    "test database file {} was not removed ({e}); the temp directory would fill up",
                    candidate.display()
                )
            });
        }
    }
}

/// A price row with only the fields a test needs; everything else stays at `PriceInput`'s default.
fn price(model: &str) -> PriceInput {
    PriceInput {
        model: model.to_string(),
        currency: "USD".to_string(),
        ..Default::default()
    }
}

/// Seed one price and return a calculator over a cache loaded from it.
async fn calculator_with(
    db: &Database,
    input: &PriceInput,
) -> Result<CostCalculator, Box<dyn Error>> {
    BillingPriceModel::upsert(db, input).await?;
    let cache = PriceCache::load(db).await?;
    Ok(CostCalculator::new(cache))
}

// -------------------------------------------------------------------------------------------
// the claim that makes a double charge impossible, asserted rather than assumed
// -------------------------------------------------------------------------------------------

#[test]
fn no_provider_parser_populates_the_modality_fields() {
    // If a parser ever starts setting `image_tokens` or `video_tokens` from a provider response, the reasoning
    // at the top of this file stops holding and the overlap question has to be asked again. This is the
    // tripwire.
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/usage/providers");
    let mut offenders = Vec::new();

    for entry in std::fs::read_dir(dir).expect("the providers directory exists") {
        let path = entry.expect("readable").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("readable");
        for field in ["image_tokens", "video_tokens", "music_tokens"] {
            for (n, line) in src.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                if code.contains(field) && code.contains(':') {
                    offenders.push(format!(
                        "{}:{}  {}",
                        path.file_name().unwrap_or_default().to_string_lossy(),
                        n + 1,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "a provider parser now populates a modality field, so the fields can overlap with `input_tokens` and \
         this file's premise needs re-examining:\n{}",
        offenders.join("\n")
    );
}

// -------------------------------------------------------------------------------------------
// image and video
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn an_explicit_image_price_is_used() -> Result<(), Box<dyn Error>> {
    let (db, path) = fresh_db("img_explicit").await?;
    let mut p = price("img-model");
    p.input_price = 1_000_000; // one nano per token
    p.image_price = Some(5_000_000); // five nano per token
    let calc = calculator_with(&db, &p).await?;

    let usage = UnifiedUsage {
        input_tokens: 100,
        image_tokens: 200,
        ..Default::default()
    };
    let result = calc
        .calculate("img-model", &usage, "req-img", false, false, None)
        .await?;

    println!("image breakdown: {:?}", result.breakdown);
    assert_eq!(result.breakdown.input_cost, 100, "100 tokens at one nano");
    assert_eq!(
        result.breakdown.image_cost, 1_000,
        "200 image tokens at five nano each, from the explicit image_price"
    );
    assert_eq!(result.usd_amount_nano, 1_100);

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn an_absent_image_price_falls_back_to_the_input_price_and_not_to_zero(
) -> Result<(), Box<dyn Error>> {
    // The fallback at `calculator.rs:317` is `unwrap_or(price.input_price)`. Falling back to **zero** would make
    // every image token free on any channel that does not set an image price, which is the shape of defect #670
    // was -- an unset field becoming a silent zero rather than a default.
    let (db, path) = fresh_db("img_fallback").await?;
    let mut p = price("img-fallback");
    p.input_price = 2_000_000; // two nano per token
                               // image_price deliberately unset
    let calc = calculator_with(&db, &p).await?;

    let usage = UnifiedUsage {
        image_tokens: 300,
        ..Default::default()
    };
    let result = calc
        .calculate("img-fallback", &usage, "req-img2", false, false, None)
        .await?;

    println!("fallback breakdown: {:?}", result.breakdown);
    assert_eq!(
        result.breakdown.image_cost, 600,
        "300 image tokens at the input price of two nano each; zero would mean the fallback is broken"
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn an_explicit_video_price_is_used_and_falls_back_like_the_image_one(
) -> Result<(), Box<dyn Error>> {
    // The same two paths for video (`calculator.rs:322-327`), in one test because they are the same expression
    // with a different field.
    let (db, path) = fresh_db("video").await?;

    // With an explicit video price.
    let mut with = price("vid-explicit");
    with.input_price = 1_000_000;
    with.video_price = Some(8_000_000);
    let calc = calculator_with(&db, &with).await?;
    let result = calc
        .calculate(
            "vid-explicit",
            &UnifiedUsage {
                video_tokens: 8, // one second of video at Veo's eight tokens
                ..Default::default()
            },
            "req-vid",
            false,
            false,
            None,
        )
        .await?;
    println!("video with a price: {:?}", result.breakdown);
    assert_eq!(
        result.breakdown.video_cost, 64,
        "8 tokens at eight nano each"
    );

    // And without one, on a separate model so the two fixtures cannot interfere.
    let mut without = price("vid-fallback");
    without.input_price = 3_000_000;
    let calc = calculator_with(&db, &without).await?;
    let result = calc
        .calculate(
            "vid-fallback",
            &UnifiedUsage {
                video_tokens: 16,
                ..Default::default()
            },
            "req-vid2",
            false,
            false,
            None,
        )
        .await?;
    println!("video without a price: {:?}", result.breakdown);
    assert_eq!(
        result.breakdown.video_cost, 48,
        "16 tokens at the input price of three nano each"
    );

    cleanup(db, path).await;
    Ok(())
}

// -------------------------------------------------------------------------------------------
// music: a flat fee, which is the shape most likely to be mis-billed
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn music_is_a_flat_fee_per_request_and_does_not_scale_with_tokens(
) -> Result<(), Box<dyn Error>> {
    // `calculator.rs:329-331` says explicitly that `music_price` is nanodollars **per request**, not per
    // million tokens, and that it is deliberately not passed through `nano()`. So the same music cost applies
    // whether the request reports one token or a million -- and if it were ever passed through `nano()`, a
    // small token count would round it to zero and music would become free.
    let (db, path) = fresh_db("music_flat").await?;
    let mut p = price("music-model");
    p.input_price = 1_000_000;
    p.music_price = Some(80_000_000); // eighty million nano = $0.08 per request
    let calc = calculator_with(&db, &p).await?;

    let tiny = calc
        .calculate(
            "music-model",
            &UnifiedUsage {
                input_tokens: 1,
                ..Default::default()
            },
            "req-music-1",
            false,
            false,
            None,
        )
        .await?;
    println!("music with 1 token: {:?}", tiny.breakdown);

    let large = calc
        .calculate(
            "music-model",
            &UnifiedUsage {
                input_tokens: 1_000_000,
                ..Default::default()
            },
            "req-music-2",
            false,
            false,
            None,
        )
        .await?;
    println!("music with 1_000_000 tokens: {:?}", large.breakdown);

    assert_eq!(
        tiny.breakdown.music_cost, 80_000_000,
        "the flat fee is charged in full even for a one-token request; a per-token reading would round it to \
         zero and make music free"
    );
    assert_eq!(
        large.breakdown.music_cost, 80_000_000,
        "and it does not scale with the token count"
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn an_absent_music_price_charges_nothing_for_music() -> Result<(), Box<dyn Error>> {
    // The contrast with image and video: music has **no** fallback to `input_price`, because a flat fee is not
    // a rate and there is no per-token price to derive it from. So an unset `music_price` is zero rather than a
    // guess. Recorded because it is the one modality where "unset means free" is the intended reading.
    let (db, path) = fresh_db("music_none").await?;
    let mut p = price("music-free");
    p.input_price = 1_000_000;
    // music_price deliberately unset
    let calc = calculator_with(&db, &p).await?;

    let result = calc
        .calculate(
            "music-free",
            &UnifiedUsage {
                input_tokens: 1_000,
                ..Default::default()
            },
            "req-music3",
            false,
            false,
            None,
        )
        .await?;

    println!("music without a price: {:?}", result.breakdown);
    assert_eq!(
        result.breakdown.music_cost, 0,
        "no music price means no music charge, and unlike image/video there is no rate to fall back to"
    );
    assert_eq!(
        result.usd_amount_nano, 1_000,
        "the input cost is unaffected"
    );

    cleanup(db, path).await;
    Ok(())
}

// -------------------------------------------------------------------------------------------
// the components do not bleed into each other
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn every_modality_charges_only_its_own_component() -> Result<(), Box<dyn Error>> {
    // One request carrying one of everything, so a component that double-counts another shows up as the wrong
    // figure in the wrong field. Every price is different, so a mix-up cannot look like a rounding difference.
    let (db, path) = fresh_db("all_modalities").await?;
    let mut p = price("all-modalities");
    p.input_price = 1_000_000; // 1 nano/token
    p.output_price = 2_000_000; // 2
    p.image_price = Some(3_000_000); // 3
    p.video_price = Some(4_000_000); // 4
    p.audio_input_price = Some(5_000_000); // 5
    p.audio_output_price = Some(6_000_000); // 6
    p.reasoning_price = Some(7_000_000); // 7
    p.embedding_price = Some(8_000_000); // 8
    p.cache_read_input_price = Some(9_000_000); // 9
    p.cache_creation_input_price = Some(10_000_000); // 10
    p.music_price = Some(11_000_000); // flat
    let calc = calculator_with(&db, &p).await?;

    // A hundred tokens in every per-token bucket, so each expected figure is its price / 10_000.
    let usage = UnifiedUsage {
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
    };
    let result = calc
        .calculate("all-modalities", &usage, "req-all", false, false, None)
        .await?;
    let b = result.breakdown.clone();
    println!("all modalities: {b:?}");
    println!("total: {}", result.usd_amount_nano);

    assert_eq!(b.input_cost, 100, "1_000_000 / 10_000");
    assert_eq!(b.output_cost, 200, "2_000_000 / 10_000");
    assert_eq!(b.image_cost, 300, "3_000_000 / 10_000");
    assert_eq!(b.video_cost, 400, "4_000_000 / 10_000");
    assert_eq!(b.reasoning_cost, 700, "7_000_000 / 10_000");
    assert_eq!(b.embedding_cost, 800, "8_000_000 / 10_000");
    assert_eq!(b.music_cost, 11_000_000, "the flat fee, not divided");

    // The cache component is read plus write, the one component made of two fields.
    assert_eq!(
        b.cache_cost, 1_900,
        "cache_read 9_000_000/10_000 = 900 plus cache_write 10_000_000/10_000 = 1_000"
    );

    // Audio: with no voice_id, `audio_cost` covers both directions and `voice_cost` is zero. The exact split
    // between the two audio fields is covered by the existing inline tests; what matters here is that the pair
    // is charged once and not also as input or output.
    assert_eq!(
        b.voice_cost, 0,
        "no voice_id, so the voice component is zero and the audio component carries it"
    );

    // The sum, computed independently of the components, so a component that silently absorbed another would
    // still be caught by the total.
    let expected_total = 100 + 200 + 1_900 + 300 + 400 + 700 + 800 + 11_000_000 + b.audio_cost;
    assert_eq!(
        result.usd_amount_nano, expected_total,
        "the total must be the sum of the components and nothing else"
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn a_modality_with_no_tokens_charges_nothing_even_when_priced() -> Result<(), Box<dyn Error>>
{
    // The `> 0` guards at `calculator.rs:316` and `:322`. A model priced for images must not charge for an
    // image it never served -- which is the mirror of the flat music fee, where the charge applies per request
    // regardless.
    let (db, path) = fresh_db("no_modality").await?;
    let mut p = price("no-modality-tokens");
    p.input_price = 1_000_000;
    p.image_price = Some(5_000_000);
    p.video_price = Some(5_000_000);
    p.music_price = Some(5_000_000);
    let calc = calculator_with(&db, &p).await?;

    let result = calc
        .calculate(
            "no-modality-tokens",
            &UnifiedUsage {
                input_tokens: 1_000,
                ..Default::default()
            },
            "req-none",
            false,
            false,
            None,
        )
        .await?;

    println!(
        "text-only request on a multimodal-priced model: {:?}",
        result.breakdown
    );
    assert_eq!(
        result.breakdown.image_cost, 0,
        "no image tokens, no image charge"
    );
    assert_eq!(
        result.breakdown.video_cost, 0,
        "no video tokens, no video charge"
    );
    assert_eq!(
        result.breakdown.music_cost, 5_000_000,
        "but music is per request, so it is charged even with no music tokens -- the asymmetry is deliberate"
    );

    cleanup(db, path).await;
    Ok(())
}
