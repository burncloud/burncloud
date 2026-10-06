#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! Streaming accumulation for `UnifiedTokenCounter` (#633, plan section 5 item 17, #667).
//!
//! The plan lists "stream accumulation" and "overflow" for this crate. `counter.rs` has inline tests and they
//! cover the shapes that are convenient: a single `set_from_usage`, two Anthropic events, an overwrite,
//! all-zero defaults, and a thread-safety smoke test. What they do not cover is what each write path actually
//! does to a value that is absent, zero or negative — and, after #667, the difference between the two ways a
//! streaming provider can report usage.
//!
//! ## The two write paths
//!
//! | method | semantics | use for |
//! | --- | --- | --- |
//! | `accumulate` | **addition** (`fetch_add`) | providers that send **incremental deltas** |
//! | `record_cumulative` | **last non-zero total wins** (`store`) | providers that resend **cumulative totals** |
//! | `set_from_usage` | full replacement | a snapshot that is authoritative for *every* field |
//!
//! Before #667 the single method `accumulate` was documented as "incremental deltas" but implemented `store`,
//! so a provider that really sent deltas would have been under-billed by the number of events, with no error
//! anywhere. The name now matches the behaviour on both sides, and `the_two_write_paths_are_not_distinguishable_only_by_name`
//! pins that.
//!
//! ## The `> 0` guard on `record_cumulative`
//!
//! A **zero** in a later event does not clear an earlier value. A field set once therefore survives every
//! subsequent event that omits it, and there is no way to reset a single field down to zero. That is
//! deliberate, because for cumulative events "absent means unchanged" is the desired reading — and it is what
//! makes Anthropic's start-then-delta pattern work at all.

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
// accumulate is addition, for providers that send deltas
// -------------------------------------------------------------------------------------------

#[test]
fn accumulate_adds_instead_of_storing() {
    // The #667 decision, measured. Three identical events leave 300, not 100 — which is what the name and the
    // documentation promise, and what a delta-sending provider needs to be billed correctly.
    let c = UnifiedTokenCounter::new();

    c.accumulate(&usage(100, 200));
    c.accumulate(&usage(100, 200));
    c.accumulate(&usage(100, 200));

    let got = c.get_usage();
    println!("after three accumulate(100, 200) calls: {got:?}");

    assert_eq!(
        (got.input_tokens, got.output_tokens),
        (300, 600),
        "accumulate is addition; if this is 100/200 the implementation has gone back to storing, and every \
         delta-sending provider would be under-billed by the number of events"
    );
}

#[test]
fn ten_delta_events_are_billed_at_their_sum() {
    // The hazard #667 named, now asserted from the fixed side: a provider streaming 10 chunks of 100 output
    // tokens has produced 1000, and must be billed for 1000 — not for the 100 that a snapshot would report.
    let c = UnifiedTokenCounter::new();
    for _ in 0..10 {
        c.accumulate(&UnifiedUsage {
            output_tokens: 100,
            ..Default::default()
        });
    }

    let got = c.get_usage();
    println!("ten delta events of 100 output tokens each: {got:?}");

    assert_eq!(
        got.output_tokens, 1000,
        "ten deltas of 100 are 1000; a store would report 100 and under-bill by 90%"
    );
}

#[test]
fn accumulate_ignores_a_zero_or_negative_delta() {
    // Two different answers for the same input, both deliberate: the counters are `u64` internally, so a
    // negative has no representation. `accumulate` skips it (the running total is not a sum of one bad event),
    // while `set_from_usage` clamps it to zero because it is authoritative for every field.
    let c = UnifiedTokenCounter::new();

    c.accumulate(&usage(100, 100));
    c.accumulate(&usage(-5, 0));

    let after_accumulate = c.get_usage();
    println!("after accumulate(-5, 0): {after_accumulate:?}");
    assert_eq!(
        (
            after_accumulate.input_tokens,
            after_accumulate.output_tokens
        ),
        (100, 100),
        "a negative or zero delta leaves the sum untouched"
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

// -------------------------------------------------------------------------------------------
// record_cumulative takes the last running total
// -------------------------------------------------------------------------------------------

#[test]
fn record_cumulative_takes_the_last_total_rather_than_the_sum() {
    // The intended use, stated positively: the final event carries the total, so the counter must end at it.
    // Adding the running totals instead would bill the request several times over.
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
        "the last delta is the total, so it wins over the earlier 100 — adding them would be 600 and would \
         over-bill the Anthropic path #667 froze"
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
    // An event that explicitly reports no input — indistinguishable, after the guard, from one that omits it.
    c.record_cumulative(&usage(0, 500));

    assert_eq!(
        c.get_usage().input_tokens,
        1500,
        "a later zero cannot reset the field; only `set_from_usage` can, and it would reset every field"
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
    // The contrast with record_cumulative, and the reason the router picks among the three by provider: a
    // snapshot is authoritative, so a field it does not mention becomes zero rather than keeping an older value.
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
// the contract #667 asked for
// -------------------------------------------------------------------------------------------

/// The contract #667 asks for: the write paths must not be distinguishable only by their names.
///
/// Both resolutions #667 listed are acceptable, and this test accepts either — but not the state it rejected,
/// where a method named `accumulate` and documented as "incremental deltas" silently did a `store`. What it
/// pins is that a caller can tell which semantics it is getting, by name and by behaviour.
#[test]
fn the_two_write_paths_are_not_distinguishable_only_by_name() {
    // Behaviour: `accumulate` adds, so two 100-token deltas are 200.
    let additive = UnifiedTokenCounter::new();
    additive.accumulate(&usage(100, 100));
    additive.accumulate(&usage(100, 100));
    let after_accumulate = additive.get_usage();

    // Behaviour: the cumulative-total path takes the last value, so two running totals end at the second.
    let cumulative = UnifiedTokenCounter::new();
    cumulative.record_cumulative(&usage(100, 100));
    cumulative.record_cumulative(&usage(200, 200));
    let after_cumulative = cumulative.get_usage();

    println!("accumulate x2: {after_accumulate:?}");
    println!("record_cumulative x2: {after_cumulative:?}");

    assert_eq!(
        (
            after_accumulate.input_tokens,
            after_accumulate.output_tokens
        ),
        (200, 200),
        "a method named `accumulate` must add"
    );
    assert_eq!(
        (
            after_cumulative.input_tokens,
            after_cumulative.output_tokens
        ),
        (200, 200),
        "and a cumulative path fed running totals must end at the last total"
    );

    // Documentation: the two are named for what they do, so a caller reading only the API cannot pick wrong.
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/counter.rs"))
        .expect("counter.rs is part of this crate");
    assert!(
        src.contains("pub fn accumulate") && src.contains("pub fn record_cumulative"),
        "both write paths must exist under names that state their semantics"
    );
    assert!(
        !src.contains("incremental deltas (Anthropic"),
        "the old doc comment claimed incremental deltas for the Anthropic path, which sends running totals"
    );
}
