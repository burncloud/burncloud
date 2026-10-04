#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    reason = "Test-only file: the assertions are the test, and the fixtures are JSON of unknown shape."
)]
//! What the provider usage parsers do with malformed or mistyped fields (#633, plan section 5 item 17).
//!
//! The plan asks for "each provider's usage with missing or malformed fields". The existing inline tests
//! cover the shapes that are *absent*: a missing `usage` object, an empty one, a missing `metadata`, and
//! malformed JSON. They do not cover the shapes that are *mistyped* -- a field that is present but is not
//! the number the parser expects.
//!
//! ## Why that class matters more than a missing field
//!
//! All five parsers read numbers as `.get(..).and_then(|v| v.as_i64()).unwrap_or(0)`. `as_i64` returns
//! `None` for anything that is not a JSON integer, so a **string**, a **float**, a **boolean** and a
//! **null** all become `0` rather than an error. `parse_response` returns `Result<UnifiedUsage, ParseError>`,
//! so the parsers have a way to report a problem and choose not to use it for this class.
//!
//! A zeroed usage is a **billing under-count**: the request is served and charged as if it used nothing,
//! and nothing downstream can tell that apart from a genuinely empty request. That is the finding these
//! tests exist to make visible.
//!
//! ## How these tests are written
//!
//! They **measure and name the behaviour**; they do not assert that it is correct. Writing
//! `assert_eq!(parsed.input_tokens, 0)` would freeze a suspected defect as the contract, which is the
//! failure mode being avoided here. Each test states what is observed and why it is or is not acceptable,
//! and the suspected defects are filed as issues rather than encoded as expectations.

use burncloud_service_billing::usage::get_parser;
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
fn a_token_count_sent_as_a_string_is_measured() {
    // `"150"` is the number the provider meant. JSON allows it, some gateways emit it, and `as_i64` returns
    // `None` for it, so the count is lost rather than recovered or reported.
    let (input, output, err) = parse(ChannelType::OpenAI, openai_body(json!("150"), json!("250")));

    println!("string counts parsed as: input={input} output={output} error={err:?}");

    // The observation, stated as a count rather than as an expectation: the tokens were dropped.
    assert_eq!(
        (input, output),
        (0, 0),
        "if this now recovers 150/250 the parser has been taught to coerce strings, which is an \
         improvement worth updating this test for"
    );
    assert_eq!(
        err, None,
        "and it is reported as success, which is what makes the loss invisible to the caller"
    );
}

#[test]
fn a_token_count_sent_as_a_float_is_measured() {
    // `150.0` is an integer written as a float. `as_i64` rejects it, so a gateway that serialises counts as
    // doubles bills zero for every request.
    let (input, output, err) = parse(ChannelType::OpenAI, openai_body(json!(150.0), json!(250.0)));

    println!("float counts parsed as: input={input} output={output} error={err:?}");
    assert_eq!((input, output), (0, 0));
    assert_eq!(err, None);
}

#[test]
fn a_token_count_sent_as_null_is_measured() {
    // A provider that knows the field but not the value may send `null`. Distinguishing "no count" from
    // "zero tokens" is impossible after this coercion.
    let (input, output, err) = parse(ChannelType::OpenAI, openai_body(json!(null), json!(null)));

    println!("null counts parsed as: input={input} output={output} error={err:?}");
    assert_eq!((input, output), (0, 0));
    assert_eq!(err, None);
}

#[test]
fn a_token_count_sent_as_a_boolean_is_measured() {
    // `true` is not a number in any reading. Recorded because it produces the same silent zero as a string,
    // which is what makes the coercion a class of behaviour rather than one special case.
    let (input, output, err) = parse(ChannelType::OpenAI, openai_body(json!(true), json!(false)));

    println!("boolean counts parsed as: input={input} output={output} error={err:?}");
    assert_eq!((input, output), (0, 0));
    assert_eq!(err, None);
}

// -------------------------------------------------------------------------------------------
// values that are numbers but not usable counts
// -------------------------------------------------------------------------------------------

#[test]
fn a_negative_token_count_is_measured() {
    // A negative count is a number, so `as_i64` accepts it and `unwrap_or(0)` does not apply -- the value
    // passes through. This is the one case in this file where the parser does **not** zero the field, and
    // the consequence is the opposite of the others: a negative input contributes a negative cost.
    let (input, output, err) = parse(ChannelType::OpenAI, openai_body(json!(-100), json!(-50)));

    println!("negative counts parsed as: input={input} output={output} error={err:?}");
    assert_eq!(err, None);
    assert_eq!(
        (input, output),
        (-100, -50),
        "negative counts are accepted as-is, so they reach the cost calculation rather than being \
         rejected or clamped"
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
fn a_string_count_is_dropped_by_every_provider() {
    // If only one parser behaved this way it would be a bug in that parser. The point of measuring all five
    // is that it is a property of the shared reading style, so a fix has to be made once and consistently.
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

    let mut dropped = Vec::new();
    let mut recovered = Vec::new();
    for (provider, channel, body) in cases {
        let (input, output, err) = parse(channel, body);
        println!("{provider}: input={input} output={output} error={err:?}");
        if (input, output) == (0, 0) {
            dropped.push(provider);
        } else {
            recovered.push(provider);
        }
    }

    println!("providers that dropped the string counts: {dropped:?}");
    println!("providers that recovered them: {recovered:?}");

    assert!(
        !dropped.is_empty(),
        "no provider dropped a string count, so the reading style has changed and this file is out of date"
    );
}

// -------------------------------------------------------------------------------------------
// the defect, filed as an issue and pinned by a failing test
// -------------------------------------------------------------------------------------------

/// The behaviour #664 asks for, which does not hold today.
///
/// **Ignored, not deleted**, and it is the failing test that #664 refers to: it states the contract the fix
/// must satisfy, so the fix has something to turn green rather than a prose description. Run it with
/// `cargo test -p burncloud-service-billing --test malformed_usage_fields -- --ignored` to see the defect.
///
/// The choice of contract is deliberate. A decimal integer string is not an ambiguous case -- `"1500"` has
/// exactly one numeric reading -- so recovering it is the least surprising behaviour and cannot silently
/// change what a correct request costs. `null`, booleans and non-numeric strings are left out of this test
/// and handled by #664's requirement to either error or warn, because for those the right answer depends on
/// a policy decision rather than on a reading.
#[ignore = "the contract for #664: a decimal integer string must be parsed, not dropped"]
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
    let _ = std::fs::remove_file(&path);
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

fn price_input(model: &str, input: i64, output: i64) -> burncloud_database_billing::PriceInput {
    burncloud_database_billing::PriceInput {
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
    use burncloud_commerce_contracts::price_u64::dollars_to_nano;
    use burncloud_database_billing::BillingPriceModel;
    use burncloud_service_billing::{CostCalculator, PriceCache};

    let (db, path) = fresh_db("string_zero_charge").await;

    let per_token = dollars_to_nano(1.0) / 1_000_000;
    BillingPriceModel::upsert(&db, &price_input("mistyped-model", per_token, per_token))
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
        usage.input_tokens, 0,
        "the string counts were dropped, so the usage handed to the calculator is empty even though the \
         provider reported 2000 tokens"
    );
    assert_eq!(
        cost.usd_amount_nano, 0,
        "and the charge is therefore zero. If this is no longer zero the parser has learned to coerce \
         strings, and this test should be updated to assert the recovered amount instead"
    );

    // The contrast that makes this a defect rather than a rounding detail: the same numbers sent as
    // integers are charged, so the only difference between the two requests is their JSON types.
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
