#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::panic_in_result_fn,
    reason = "Test-only file: the assertions are the test, the fixtures are JSON of unknown shape, and clippy.toml's allow-panic-in-tests does not recognise #[tokio::test]"
)]
//! What the provider usage parsers do with malformed or mistyped fields (#633, plan section 5 item 17).
//!
//! The plan asks for "each provider's usage with missing or malformed fields". The existing inline tests
//! cover the shapes that are *absent*: a missing `usage` object, an empty one, a missing `metadata`, and
//! malformed JSON. They did not cover the shapes that are *mistyped* -- a field that is present but is not
//! the number the parser expects.
//!
//! ## What was found, and what changed
//!
//! All five parsers read numbers as `.get(..).and_then(|v| v.as_i64()).unwrap_or(0)`, and `as_i64` returns
//! `None` for anything that is not a JSON integer. A **string**, a **float**, a **boolean** and a **null**
//! therefore became `0`, reported as a successful parse. Measured end to end: a request that consumed 2000
//! tokens was billed **zero**, indistinguishable from a free request. Filed as #664.
//!
//! The fix is a shared `read_count` in [`crate::usage`], used by all five parsers so they cannot drift
//! apart on this. A decimal integer string, a float with no fraction, and `"1500.0"` are now read; `null`,
//! booleans, non-numeric strings and fractional floats are **reported** as
//! [`crate::error::ParseError::MalformedField`]; an absent field stays `0`, which is the documented shape
//! for a response that carries no usage.
//!
//! ## What this file still records rather than fixes
//!
//! `parse_response_or_default` maps `Err` to `UnifiedUsage::default()`, which is zero, so a malformed count
//! still ends up billed as nothing one layer up -- with a warning where there used to be silence. The
//! parsers now say "this count is unreadable"; what the **caller** should do instead of billing zero is a
//! decision about refusing a request or billing an estimate, and `a_malformed_count_still_becomes_a_zero_charge_at_the_caller`
//! records that boundary so it is not mistaken for a solved problem.
//!
//! ## How these tests are written
//!
//! Each names the behaviour and the reason it is or is not acceptable, and the amounts are derived by hand
//! rather than read off a run -- the plan asks for the expected amount to be independently calculated.

use burncloud_commerce_billing::usage::get_parser;
use burncloud_supply_contracts::ChannelType;
use serde_json::json;

/// Parse a response body, returning `(input, output, error)`.
///
/// `get_parser` always returns a parser -- it falls back to `GenericParser` for any channel type without a
/// dedicated one -- so there is no "no parser" case to handle here.
fn parse(channel: ChannelType, body: serde_json::Value) -> (i64, i64, Option<String>) {
    let parser = get_parser(channel);
    match parser.parse_response(&body) {
        Ok(u) => (u.input_tokens, u.output_tokens, None),
        Err(e) => (0, 0, Some(format!("{e}"))),
    }
}

/// A well-formed OpenAI response, used as the control for every mistyping below.
fn openai_body(prompt: serde_json::Value, completion: serde_json::Value) -> serde_json::Value {
    json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "usage": { "prompt_tokens": prompt, "completion_tokens": completion, "total_tokens": 0 }
    })
}

// -------------------------------------------------------------------------------------------
// the control: the shapes the existing tests already cover, so a failure here means the fixture
// -------------------------------------------------------------------------------------------

#[test]
fn well_formed_integers_are_parsed() {
    // Without this, a zero everywhere below could be blamed on the fixture rather than on the mistake.
    let (input, output, err) = parse(ChannelType::OpenAI, openai_body(json!(150), json!(250)));
    assert_eq!(err, None);
    assert_eq!((input, output), (150, 250));
}

#[test]
fn an_absent_usage_object_yields_zero_without_an_error() {
    // Covered by the inline tests already; repeated here so the measurement file is self-contained and the
    // difference between "absent" (documented as zero) and "mistyped" (measured below) is visible.
    let (input, output, err) = parse(ChannelType::OpenAI, json!({ "id": "x", "choices": [] }));
    assert_eq!(err, None);
    assert_eq!((input, output), (0, 0));
}

// -------------------------------------------------------------------------------------------
// mistyped numbers, measured
// -------------------------------------------------------------------------------------------

#[test]
fn a_token_count_sent_as_a_string_is_parsed() {
    // `"150"` is the number the provider meant, and JSON allows it. This used to be dropped to zero and
    // reported as success; #664 changed that, and this test was updated with it.
    let (input, output, err) = parse(ChannelType::OpenAI, openai_body(json!("150"), json!("250")));

    println!("string counts parsed as: input={input} output={output} error={err:?}");
    assert_eq!(err, None);
    assert_eq!(
        (input, output),
        (150, 250),
        "a decimal integer string has exactly one numeric reading, so recovering it cannot change what a \
         correct request costs"
    );
}

#[test]
fn a_token_count_sent_as_a_float_with_no_fraction_is_parsed() {
    // `150.0` is an integer written as a float, so it has one numeric reading. A gateway that serialises
    // counts as doubles used to bill zero for every request.
    let (input, output, err) = parse(ChannelType::OpenAI, openai_body(json!(150.0), json!(250.0)));

    println!("float counts parsed as: input={input} output={output} error={err:?}");
    assert_eq!(err, None);
    assert_eq!((input, output), (150, 250));
}

#[test]
fn a_token_count_sent_as_a_float_with_a_fraction_is_reported() {
    // The boundary of that tolerance. `150.5` has no single reading as a token count, and rounding would
    // invent a number the upstream did not report, so it is reported instead.
    let (input, output, err) = parse(ChannelType::OpenAI, openai_body(json!(150.5), json!(250)));

    println!("fractional float parsed as: input={input} output={output} error={err:?}");
    assert!(
        err.as_deref()
            .is_some_and(|e| e.contains("Malformed field")),
        "a fractional token count must be reported, got {err:?}"
    );
}

#[test]
fn a_token_count_sent_as_null_is_reported() {
    // A provider that knows the field but not the value may send `null`. "Unknown count" is not "zero
    // tokens", so it is reported rather than billed as nothing.
    let (input, output, err) = parse(ChannelType::OpenAI, openai_body(json!(null), json!(null)));

    println!("null counts parsed as: input={input} output={output} error={err:?}");
    assert!(
        err.as_deref()
            .is_some_and(|e| e.contains("Malformed field") && e.contains("prompt_tokens")),
        "a null count must be reported and must name the field, got {err:?}"
    );
}

#[test]
fn a_token_count_sent_as_a_boolean_is_reported() {
    // `true` is not a number in any reading.
    let (input, output, err) = parse(ChannelType::OpenAI, openai_body(json!(true), json!(false)));

    println!("boolean counts parsed as: input={input} output={output} error={err:?}");
    assert!(
        err.as_deref()
            .is_some_and(|e| e.contains("Malformed field")),
        "a boolean count must be reported, got {err:?}"
    );
}

// -------------------------------------------------------------------------------------------
// values that are numbers but not usable counts
// -------------------------------------------------------------------------------------------

#[test]
fn a_negative_input_count_is_clamped_and_a_negative_output_count_is_not() {
    // This test used to assert that both negative counts passed through as `-100` and `-50`, and it passed.
    // The de-duplication fix in #670 changed the input side, and the change is an improvement.
    //
    // The input count now goes through the same clamp as the cached-token subtraction, so a negative
    // `prompt_tokens` becomes **0** instead of a negative number reaching the cost calculation. That clamp
    // was written for the cache split and applies here as a side effect -- a welcome one, since a negative
    // input count would *reduce* a charge.
    //
    // The **output** count has no such path and still passes through negative, and the calculator's own guard
    // (`calculator.rs:350`, which logs "negative token count ... treating as 0") is what handles it. So the
    // two fields are treated differently, which is worth recording rather than leaving to be discovered:
    // negative counts are caught in two different places depending on which field they arrive in.
    let (input, output, err) = parse(ChannelType::OpenAI, openai_body(json!(-100), json!(-50)));

    println!("negative counts parsed as: input={input} output={output} error={err:?}");
    assert_eq!(
        err, None,
        "a negative is a number, so it is not reported as malformed"
    );
    assert_eq!(
        input, 0,
        "the input side is clamped by the same expression that splits out the cached tokens"
    );
    assert_eq!(
        output, -50,
        "and the output side still passes through, relying on the calculator's guard rather than the parser's"
    );
    assert!(
        output < 0,
        "which means a negative output count does reach the cost calculation, where calculator.rs:350 \
         zeroes it with a warning instead of the parser refusing it"
    );
}

#[test]
fn an_enormous_token_count_is_measured() {
    // Overflow: the largest i64 is accepted, and how it behaves in the cost calculation is the next crate
    // over, but the parser's decision to accept it is recorded here.
    let (input, output, err) = parse(
        ChannelType::OpenAI,
        openai_body(json!(i64::MAX), json!(i64::MAX)),
    );

    println!("i64::MAX counts parsed as: input={input} output={output} error={err:?}");
    assert_eq!(err, None);
    assert_eq!((input, output), (i64::MAX, i64::MAX));

    // One past the maximum is not representable in JSON as an integer; serde_json parses it as a float or
    // fails, which lands in the float case above. Recorded so the boundary is not assumed.
    let beyond = serde_json::from_str::<serde_json::Value>("{\"n\": 9223372036854775808}");
    println!("one past i64::MAX parses as: {beyond:?}");
}

// -------------------------------------------------------------------------------------------
// the same mistake across the other providers, so this is a class rather than an OpenAI quirk
// -------------------------------------------------------------------------------------------

#[test]
fn a_string_count_is_recovered_by_every_provider() {
    // The defect was shared -- all five read counts the same way -- so this checks that the fix is shared
    // too. If one provider recovered strings and another did not, the others were left behind.
    let cases: Vec<(&str, ChannelType, serde_json::Value)> = vec![
        (
            "openai",
            ChannelType::OpenAI,
            json!({ "usage": { "prompt_tokens": "150", "completion_tokens": "250" } }),
        ),
        (
            "anthropic",
            ChannelType::Anthropic,
            json!({ "usage": { "input_tokens": "150", "output_tokens": "250" } }),
        ),
        (
            "deepseek",
            ChannelType::DeepSeek,
            json!({ "usage": { "prompt_tokens": "150", "completion_tokens": "250" } }),
        ),
        (
            "gemini",
            ChannelType::Gemini,
            json!({ "usageMetadata": { "promptTokenCount": "150", "candidatesTokenCount": "250" } }),
        ),
        // `get_parser` falls back to `GenericParser` for a channel type with no dedicated parser, so this
        // case is reached with `Unknown` rather than by asking for a parser that does not exist.
        (
            "generic",
            ChannelType::Unknown,
            json!({ "usage": { "prompt_tokens": "150", "completion_tokens": "250" } }),
        ),
    ];

    let mut wrong = Vec::new();
    for (provider, channel, body) in cases {
        let (input, output, err) = parse(channel, body);
        println!("{provider}: input={input} output={output} error={err:?}");
        if (input, output) != (150, 250) {
            wrong.push(format!("{provider}: got {input}/{output} (error {err:?})"));
        }
    }

    assert!(
        wrong.is_empty(),
        "these providers did not recover the string counts: {wrong:?}"
    );
}

// -------------------------------------------------------------------------------------------
// the defect, filed as an issue and pinned by a failing test
// -------------------------------------------------------------------------------------------

/// The contract #664 asked for, now satisfied.
///
/// This was written **failing** and marked `#[ignore]` so the fix would have something to turn green rather
/// than a prose description, and the fix has landed. It is kept as a regression guard: it is the test that
/// fails if any of the five parsers goes back to reading counts with `as_i64().unwrap_or(0)`.
///
/// The contract is deliberately narrow. A decimal integer string is not an ambiguous case -- `"1500"` has
/// exactly one numeric reading -- so recovering it is the least surprising behaviour and cannot silently
/// change what a correct request costs. `null`, booleans and non-numeric strings are handled by the tests
/// below, which require them to be **reported** rather than zeroed.
#[test]
fn a_decimal_integer_string_is_parsed_rather_than_dropped() {
    let cases: Vec<(&str, ChannelType, serde_json::Value)> = vec![
        (
            "openai",
            ChannelType::OpenAI,
            json!({ "usage": { "prompt_tokens": "150", "completion_tokens": "250" } }),
        ),
        (
            "anthropic",
            ChannelType::Anthropic,
            json!({ "usage": { "input_tokens": "150", "output_tokens": "250" } }),
        ),
        (
            "deepseek",
            ChannelType::DeepSeek,
            json!({ "usage": { "prompt_tokens": "150", "completion_tokens": "250" } }),
        ),
        (
            "gemini",
            ChannelType::Gemini,
            json!({ "usageMetadata": { "promptTokenCount": "150", "candidatesTokenCount": "250" } }),
        ),
        (
            "generic",
            ChannelType::Unknown,
            json!({ "usage": { "prompt_tokens": "150", "completion_tokens": "250" } }),
        ),
    ];

    let mut wrong = Vec::new();
    for (provider, channel, body) in cases {
        let (input, output, err) = parse(channel, body);
        println!("{provider}: input={input} output={output} error={err:?}");
        if (input, output) != (150, 250) {
            wrong.push(format!("{provider}: got {input}/{output}"));
        }
    }

    assert!(
        wrong.is_empty(),
        "these providers dropped the string counts instead of reading them: {wrong:?}"
    );
}

// -------------------------------------------------------------------------------------------
// the boundary that is still open
// -------------------------------------------------------------------------------------------

#[test]
fn a_malformed_count_still_becomes_a_zero_charge_at_the_caller() {
    // The parsers now report an unreadable count, but the function the router actually calls does not
    // propagate that: `parse_response_or_default` maps `Err` to `UnifiedUsage::default()`, which is zero.
    //
    // So the under-count from #664 is **narrowed, not closed**: a string count is recovered and billed
    // correctly, while a `null` or non-numeric count still ends up billed as nothing -- now with a
    // `tracing::warn!` where there used to be silence.
    //
    // Recorded as a test rather than as prose because it is the part a reader is most likely to assume was
    // fixed along with the rest. Closing it needs a decision about what the caller should do instead of
    // zero: refuse the request, or bill at an estimate. That is #664's remaining half.
    use burncloud_commerce_billing::usage::parse_response_or_default;

    let parser = get_parser(ChannelType::OpenAI);

    // The unreadable case: reported by the parser...
    let malformed = json!({ "usage": { "prompt_tokens": null, "completion_tokens": 250 } });
    assert!(
        parser.parse_response(&malformed).is_err(),
        "the parser must report this, which is the part that was fixed"
    );

    // ...and turned into zero by the caller, which is the part that was not.
    let usage = parse_response_or_default(parser.as_ref(), &malformed, "req-malformed");
    println!(
        "caller received: input={} output={}",
        usage.input_tokens, usage.output_tokens
    );
    assert_eq!(
        usage.input_tokens, 0,
        "the caller still receives zero for an unreadable count. The 250 that WAS readable is lost too, \
         because the whole response falls back to a default -- so one bad field discards the good ones"
    );
    assert_eq!(
        usage.output_tokens, 0,
        "and the readable output count is discarded along with it, which is a second way this loses money"
    );

    // The contrast: the recoverable case now reaches the caller intact.
    let recoverable = json!({ "usage": { "prompt_tokens": "1500", "completion_tokens": "500" } });
    let usage = parse_response_or_default(parser.as_ref(), &recoverable, "req-recoverable");
    assert_eq!(
        (usage.input_tokens, usage.output_tokens),
        (1500, 500),
        "a string count now survives the caller, which is the half that was fixed"
    );
}

// -------------------------------------------------------------------------------------------
// the consequence, end to end: a mistyped count reaches the cost calculation as a zero charge
// -------------------------------------------------------------------------------------------

/// A fresh SQLite file database with the real migrations applied. Three slashes because the path is
/// absolute; the two-slash form is not a URL SQLite can open on Windows.
async fn fresh_db(tag: &str) -> (burncloud_database::Database, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "bc_malformed_{}_{}_{}.db",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::remove_file(&path).ok();
    let normalized = path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{}?mode=rwc", normalized);
    let db = burncloud_database::create_database_with_url(&url)
        .await
        .unwrap_or_else(|e| panic!("open {url}: {e}"));
    (db, path)
}

/// Remove the database and its WAL companions.
fn remove_db_files(path: &std::path::Path) {
    for suffix in ["", "-wal", "-shm"] {
        let mut candidate = path.to_path_buf().into_os_string();
        candidate.push(suffix);
        let candidate = std::path::PathBuf::from(candidate);
        if candidate.exists() {
            std::fs::remove_file(&candidate)
                .unwrap_or_else(|e| panic!("{} was not removed ({e})", candidate.display()));
        }
    }
}

fn price_input(model: &str, input: i64, output: i64) -> burncloud_commerce_billing::PriceInput {
    burncloud_commerce_billing::PriceInput {
        model: model.to_string(),
        input_price: input,
        output_price: output,
        currency: "USD".to_string(),
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
        voices_pricing: None,
        video_pricing: None,
        asr_pricing: None,
        realtime_pricing: None,
        model_type: None,
    }
}

#[tokio::test]
async fn a_string_token_count_produces_a_zero_charge_for_a_non_empty_request(
) -> Result<(), Box<dyn std::error::Error>> {
    // The finding the measurements above add up to, stated as money rather than as a zero -- because "the
    // parser returned 0" is not by itself a defect: a genuinely empty request also returns 0.
    //
    // The request below is **not** empty. The provider reported 2000 tokens and sent the counts as strings.
    // The parser drops both, so the cost calculation is handed a zero usage and charges nothing. No error is
    // reported anywhere, and the resulting charge is indistinguishable from a free request.
    //
    // The price is chosen so the difference between "charged" and "not charged" is a number that cannot be
    // confused with rounding: 2000 tokens at $1 per million is 2000 nano-dollars.
    use burncloud_commerce_billing::BillingPriceModel;
    use burncloud_commerce_billing::{CostCalculator, PriceCache};

    let (db, path) = fresh_db("string_zero_charge").await;

    // `input_price` is **nano-dollars per million tokens** (`calculator.rs:349`:
    // `cost_nano = tokens * price_per_million / 1_000_000`), so 1_000_000 means one nano-dollar per token and
    // 2000 tokens cost 2_000_000 nano-dollars.
    //
    // Two mistakes were made here before this comment. The first computed the price as
    // `dollars_to_nano(1.0) / 1_000_000`, which is integer division and truncates to 0. The second read the
    // field as per-token rather than per-million and set 1000. The exact-figure assertion below caught both:
    // it came out as 1 each time instead of 2_000_000.
    let price_per_million: i64 = 1_000_000;
    BillingPriceModel::upsert(
        &db,
        &price_input("mistyped-model", price_per_million, price_per_million),
    )
    .await
    .expect("seed price");

    let cache = PriceCache::load(&db).await?;
    let calc = CostCalculator::new(cache);

    // What the provider sent: the right numbers, as strings.
    let provider_response = json!({
        "id": "chatcmpl-mistyped",
        "usage": { "prompt_tokens": "1500", "completion_tokens": "500", "total_tokens": "2000" }
    });

    let parser = get_parser(ChannelType::OpenAI);
    let usage = parser
        .parse_response(&provider_response)
        .expect("the parser reports success for this body");

    println!(
        "parsed usage from string counts: input={} output={}",
        usage.input_tokens, usage.output_tokens
    );

    let cost = calc
        .calculate("mistyped-model", &usage, "req-mistyped", false, false, None)
        .await
        .expect("the model is priced, so this must not be a PriceNotFound");

    println!("charged for a 2000-token request: {cost:?}");

    assert_eq!(
        usage.input_tokens, 1500,
        "the string counts must be recovered rather than dropped -- this was the defect in #664, where the \
         usage handed to the calculator was empty even though the provider reported 2000 tokens"
    );
    assert_eq!(usage.output_tokens, 500);

    // The expected charge, worked out by hand rather than read off a run, because the plan asks for the
    // amount to be independently derived.
    //
    // `input_price` is nano-dollars per **million** tokens and the cost is
    // `tokens * input_price / 1_000_000` (`calculator.rs:349`). With `input_price = 1_000_000` the same
    // division appears on both sides and cancels, leaving `tokens` nano-dollars: 1500 input + 500 output =
    // 2000 nano-dollars. The breakdown assertions below check that split rather than only the total, so a
    // charge that came from one of the two counts twice would fail.
    assert_eq!(
        cost.usd_amount_nano, 2000,
        "1500 input + 500 output at one nano-dollar per token. Before #664 both counts were dropped and \
         this was 0"
    );
    assert_eq!(
        cost.breakdown.input_cost, 1500,
        "the input half of the split"
    );
    assert_eq!(cost.breakdown.output_cost, 500, "the output half");

    // The contrast that made it a defect rather than a rounding detail: the same numbers sent as integers
    // are charged the same, so the two requests now differ in nothing but their JSON types.
    let as_integers = parse(
        ChannelType::OpenAI,
        json!({ "usage": { "prompt_tokens": 1500, "completion_tokens": 500 } }),
    );
    assert_eq!(
        (as_integers.0, as_integers.1),
        (1500, 500),
        "the integer form parses, so the loss is caused by the JSON type and nothing else"
    );

    db.close().await.ok();
    // `close` releases the pool asynchronously, so removing the file immediately fails on Windows with
    // os error 32 ("being used by another process"). Same wait the other tests in this crate need.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    remove_db_files(&path);

    Ok(())
}
