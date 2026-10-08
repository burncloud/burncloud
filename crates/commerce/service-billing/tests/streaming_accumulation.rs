#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! Streaming cumulative-usage semantics for `UnifiedTokenCounter` (#633, #667).
//!
//! `record_cumulative` is intentionally **not addition**. It records the last
//! positive total seen for each field, which matches Anthropic streaming usage:
//! `message_start` carries the input side and `message_delta` carries a
//! cumulative output total. A zero or negative field means "no update".
//!
//! `set_from_usage` remains the full-snapshot path used when every field in a
//! usage object is authoritative. The two names now expose the semantic
//! difference directly, removing the old `accumulate` naming trap without
//! changing Anthropic billing behavior.

use burncloud_service_billing::counter::UnifiedTokenCounter;
use burncloud_service_billing::types::UnifiedUsage;

fn usage(input: i64, output: i64) -> UnifiedUsage {
    UnifiedUsage {
        input_tokens: input,
        output_tokens: output,
        ..Default::default()
    }
}

/// Every count field, so a test can assert on all ten without repeating them.
fn all_fields(u: &UnifiedUsage) -> [(&'static str, i64); 10] {
    [
        ("input_tokens", u.input_tokens),
        ("output_tokens", u.output_tokens),
        ("cache_read_tokens", u.cache_read_tokens),
        ("cache_write_tokens", u.cache_write_tokens),
        ("audio_input_tokens", u.audio_input_tokens),
        ("audio_output_tokens", u.audio_output_tokens),
        ("image_tokens", u.image_tokens),
        ("video_tokens", u.video_tokens),
        ("reasoning_tokens", u.reasoning_tokens),
        ("embedding_tokens", u.embedding_tokens),
    ]
}

// -------------------------------------------------------------------------------------------
// record_cumulative is a snapshot, not a sum
// -------------------------------------------------------------------------------------------

#[test]
fn record_cumulative_stores_rather_than_adds() {
    // The finding. Three identical events leave 100, not 300, which is correct for Anthropic's cumulative
    // events and catastrophic for a provider that sends deltas.
    let c = UnifiedTokenCounter::new();

    c.record_cumulative(&usage(100, 200));
    c.record_cumulative(&usage(100, 200));
    c.record_cumulative(&usage(100, 200));

    let got = c.get_usage();
    println!("after three record_cumulative(100, 200) calls: {got:?}");

    assert_eq!(
        (got.input_tokens, got.output_tokens),
        (100, 200),
        "record_cumulative stores the last non-zero value; it does not sum. If this ever becomes 300 the semantics \
         have changed to addition, and every Anthropic stream would then be billed a multiple of its usage"
    );
}

#[test]
fn record_cumulative_takes_the_last_cumulative_total() {
    // The intended use, stated positively: the final event carries the total, so the counter must end at it.
    let c = UnifiedTokenCounter::new();

    // message_start carries the input side.
    c.record_cumulative(&UnifiedUsage {
        input_tokens: 1500,
        cache_read_tokens: 400,
        ..Default::default()
    });
    // message_delta carries the running output total, twice, growing.
    c.record_cumulative(&UnifiedUsage {
        output_tokens: 100,
        ..Default::default()
    });
    c.record_cumulative(&UnifiedUsage {
        output_tokens: 500,
        ..Default::default()
    });

    let got = c.get_usage();
    println!("after a start and two deltas: {got:?}");

    assert_eq!(
        got.input_tokens, 1500,
        "the start's count survives the deltas"
    );
    assert_eq!(got.cache_read_tokens, 400, "and so does its cache count");
    assert_eq!(
        got.output_tokens, 500,
        "the last delta is the total, so it wins over the earlier 100"
    );
}

#[test]
fn a_zero_in_a_later_event_preserves_the_earlier_value() {
    // The `> 0` guard. A field omitted from a later event keeps its value, which is what makes the
    // start-then-delta pattern work at all.
    let c = UnifiedTokenCounter::new();

    c.record_cumulative(&usage(1500, 0));
    c.record_cumulative(&usage(0, 500));

    let got = c.get_usage();
    assert_eq!(
        got.input_tokens, 1500,
        "a zero input must not clear the earlier 1500"
    );
    assert_eq!(got.output_tokens, 500);
}

#[test]
fn a_field_set_once_cannot_be_reset_to_zero_by_a_later_event() {
    // The consequence of that guard, and a real limitation: there is no way to say "this field is now zero".
    // For cumulative events that is correct, because the upstream never reports a count going back to zero
    // within one stream. Recorded so it is not mistaken for an oversight.
    let c = UnifiedTokenCounter::new();

    c.record_cumulative(&usage(0, 0));
    c.record_cumulative(&usage(1500, 0));
    // An event that explicitly reports no input -- indistinguishable, after the guard, from one that omits it.
    c.record_cumulative(&usage(0, 500));

    assert_eq!(
        c.get_usage().input_tokens,
        1500,
        "a later zero cannot reset the field; only `set_from_usage` can, and it would reset every field"
    );
}

#[test]
fn a_negative_count_is_ignored_by_record_cumulative_and_clamped_by_set_from_usage() {
    // Two methods, two different answers for the same input. This is worth pinning because the counters are
    // `u64` internally, so a negative cannot be stored either way -- but one path silently keeps the old
    // value while the other silently writes zero, and a caller cannot tell which happened.
    let c = UnifiedTokenCounter::new();

    c.record_cumulative(&usage(100, 100));
    c.record_cumulative(&usage(-5, -5));

    let after_record_cumulative = c.get_usage();
    println!("after record_cumulative(-5, -5): {after_record_cumulative:?}");
    assert_eq!(
        (
            after_record_cumulative.input_tokens,
            after_record_cumulative.output_tokens
        ),
        (100, 100),
        "a negative is skipped by record_cumulative, leaving the previous value in place"
    );

    c.set_from_usage(&usage(-5, -5));
    let after_set = c.get_usage();
    println!("after set_from_usage(-5, -5): {after_set:?}");
    assert_eq!(
        (after_set.input_tokens, after_set.output_tokens),
        (0, 0),
        "while set_from_usage clamps it to zero, discarding the 100 that was there"
    );
}

#[test]
fn the_guard_behaves_the_same_way_for_all_ten_fields() {
    // One field behaving differently from the other nine would be the kind of defect that only shows up for
    // audio or video requests, so the guard is asserted across the whole struct rather than on two fields.
    let c = UnifiedTokenCounter::new();

    let full = UnifiedUsage {
        input_tokens: 1,
        output_tokens: 2,
        cache_read_tokens: 3,
        cache_write_tokens: 4,
        audio_input_tokens: 5,
        audio_output_tokens: 6,
        image_tokens: 7,
        video_tokens: 8,
        reasoning_tokens: 9,
        embedding_tokens: 10,
    };
    c.record_cumulative(&full);
    for (name, got) in all_fields(&c.get_usage()) {
        assert!(got > 0, "{name} was not stored from a full event");
    }

    // An all-zero event must leave every one of them untouched.
    c.record_cumulative(&UnifiedUsage::default());
    for (name, got) in all_fields(&c.get_usage()) {
        assert!(got > 0, "{name} was cleared by an all-zero event");
    }

    // A negative on every field must also leave them untouched.
    let negative = UnifiedUsage {
        input_tokens: -1,
        output_tokens: -2,
        cache_read_tokens: -3,
        cache_write_tokens: -4,
        audio_input_tokens: -5,
        audio_output_tokens: -6,
        image_tokens: -7,
        video_tokens: -8,
        reasoning_tokens: -9,
        embedding_tokens: -10,
    };
    c.record_cumulative(&negative);
    let expected = all_fields(&full);
    let actual = all_fields(&c.get_usage());
    assert_eq!(
        actual, expected,
        "a negative on every field must leave all ten as they were"
    );
}

// -------------------------------------------------------------------------------------------
// set_from_usage is a full replacement
// -------------------------------------------------------------------------------------------

#[test]
fn set_from_usage_clears_fields_the_new_snapshot_omits() {
    // The contrast with record_cumulative, and the reason the router picks between them by provider: a snapshot is
    // authoritative, so a field it does not mention becomes zero rather than keeping an older value.
    let c = UnifiedTokenCounter::new();

    c.set_from_usage(&usage(1500, 500));
    assert_eq!(c.get_usage().input_tokens, 1500);

    // A snapshot carrying only output.
    c.set_from_usage(&usage(0, 500));

    let got = c.get_usage();
    assert_eq!(
        got.input_tokens, 0,
        "a snapshot replaces everything, so the earlier input count is gone"
    );
    assert_eq!(got.output_tokens, 500);
}

#[test]
fn set_from_usage_overwrites_every_field_including_the_ones_record_cumulative_would_keep() {
    let c = UnifiedTokenCounter::new();

    c.set_from_usage(&UnifiedUsage {
        cache_read_tokens: 400,
        audio_input_tokens: 70,
        ..Default::default()
    });
    assert_eq!(c.get_usage().cache_read_tokens, 400);
    assert_eq!(c.get_usage().audio_input_tokens, 70);

    c.set_from_usage(&UnifiedUsage::default());

    for (name, got) in all_fields(&c.get_usage()) {
        assert_eq!(got, 0, "{name} survived a default snapshot");
    }
}

// -------------------------------------------------------------------------------------------
// #667 resolved: cumulative API is explicit and additive semantics are not implied
// -------------------------------------------------------------------------------------------

#[test]
fn record_cumulative_is_not_for_delta_providers() {
    // Ten 100-token values are ten *cumulative snapshots* of the same 100-token
    // total, so this API intentionally ends at 100. A provider that sends true
    // deltas must use a separately named additive API if/when one is introduced.
    let c = UnifiedTokenCounter::new();
    for _ in 0..10 {
        c.record_cumulative(&UnifiedUsage {
            output_tokens: 100,
            ..Default::default()
        });
    }

    assert_eq!(
        c.get_usage().output_tokens,
        100,
        "record_cumulative keeps the latest positive total; it must never imply delta addition"
    );
}

#[test]
fn the_misleading_accumulate_api_is_gone() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/counter.rs"))
        .unwrap_or_else(|e| panic!("failed to read counter.rs: {e}"));

    assert!(
        src.contains("pub fn record_cumulative("),
        "the cumulative-total API must be explicitly named"
    );
    assert!(
        !src.contains("pub fn accumulate("),
        "#667 is unresolved if a storing method is still exposed as accumulate"
    );
}
