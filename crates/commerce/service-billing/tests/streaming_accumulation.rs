#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! Streaming accumulation for `UnifiedTokenCounter` (#633, plan section 5 item 17).
//!
//! The plan lists "stream accumulation" and "overflow" for this crate. `counter.rs` has five inline tests
//! and they cover the shapes that are convenient: a single `set_from_usage`, two Anthropic events, an
//! overwrite, all-zero defaults, and a thread-safety smoke test. What they do not cover is what the two
//! write paths actually do to a value that is absent, zero or negative.
//!
//! ## What this file establishes
//!
//! `accumulate` does **not** add. It stores:
//!
//! ```ignore
//! if usage.output_tokens > 0 {
//!     self.output_tokens.store(usage.output_tokens as u64, Ordering::Relaxed);
//! }
//! ```
//!
//! That is "the last non-zero value wins", and calling it three times with `100`, `100`, `100` leaves **100**,
//! not 300. For Anthropic -- the only caller (`lib.rs:3527`, `lib.rs:3817`) -- that is the **correct**
//! reading, because its `message_start` and `message_delta` events carry **cumulative totals** rather than
//! deltas. So the behaviour is right for the provider it serves.
//!
//! What is not right is the name and the documentation. `accumulate` reads as addition and
//! `counter.rs:54` says "incremental deltas"; a provider that really sent deltas would be under-billed by
//! whatever factor the number of events happens to be, with no error anywhere. That is filed as #667 and
//! pinned by a failing test at the end of this file.
//!
//! ## The other thing worth knowing
//!
//! A **zero** in a later event does not clear an earlier value: the `> 0` guard skips it. A field that was
//! set once therefore survives every subsequent event that omits it, and there is no way to reset a single
//! field down to zero. Recorded rather than judged, because for cumulative events "absent means unchanged"
//! is the desired reading.

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
// accumulate is a snapshot, not a sum
// -------------------------------------------------------------------------------------------

#[test]
fn accumulate_stores_rather_than_adds() {
    // The finding. Three identical events leave 100, not 300, which is correct for Anthropic's cumulative
    // events and catastrophic for a provider that sends deltas.
    let c = UnifiedTokenCounter::new();

    c.accumulate(&usage(100, 200));
    c.accumulate(&usage(100, 200));
    c.accumulate(&usage(100, 200));

    let got = c.get_usage();
    println!("after three accumulate(100, 200) calls: {got:?}");

    assert_eq!(
        (got.input_tokens, got.output_tokens),
        (100, 200),
        "accumulate stores the last non-zero value; it does not sum. If this ever becomes 300 the semantics \
         have changed to addition, and every Anthropic stream would then be billed a multiple of its usage"
    );
}

#[test]
fn accumulate_takes_the_last_cumulative_total() {
    // The intended use, stated positively: the final event carries the total, so the counter must end at it.
    let c = UnifiedTokenCounter::new();

    // message_start carries the input side.
    c.accumulate(&UnifiedUsage {
        input_tokens: 1500,
        cache_read_tokens: 400,
        ..Default::default()
    });
    // message_delta carries the running output total, twice, growing.
    c.accumulate(&UnifiedUsage {
        output_tokens: 100,
        ..Default::default()
    });
    c.accumulate(&UnifiedUsage {
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

    c.accumulate(&usage(1500, 0));
    c.accumulate(&usage(0, 500));

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

    c.accumulate(&usage(0, 0));
    c.accumulate(&usage(1500, 0));
    // An event that explicitly reports no input -- indistinguishable, after the guard, from one that omits it.
    c.accumulate(&usage(0, 500));

    assert_eq!(
        c.get_usage().input_tokens,
        1500,
        "a later zero cannot reset the field; only `set_from_usage` can, and it would reset every field"
    );
}

#[test]
fn a_negative_count_is_ignored_by_accumulate_and_clamped_by_set_from_usage() {
    // Two methods, two different answers for the same input. This is worth pinning because the counters are
    // `u64` internally, so a negative cannot be stored either way -- but one path silently keeps the old
    // value while the other silently writes zero, and a caller cannot tell which happened.
    let c = UnifiedTokenCounter::new();

    c.accumulate(&usage(100, 100));
    c.accumulate(&usage(-5, -5));

    let after_accumulate = c.get_usage();
    println!("after accumulate(-5, -5): {after_accumulate:?}");
    assert_eq!(
        (
            after_accumulate.input_tokens,
            after_accumulate.output_tokens
        ),
        (100, 100),
        "a negative is skipped by accumulate, leaving the previous value in place"
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
    c.accumulate(&full);
    for (name, got) in all_fields(&c.get_usage()) {
        assert!(got > 0, "{name} was not stored from a full event");
    }

    // An all-zero event must leave every one of them untouched.
    c.accumulate(&UnifiedUsage::default());
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
    c.accumulate(&negative);
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
    // The contrast with accumulate, and the reason the router picks between them by provider: a snapshot is
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
fn set_from_usage_overwrites_every_field_including_the_ones_accumulate_would_keep() {
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
// the boundary that is still open
// -------------------------------------------------------------------------------------------

#[test]
fn accumulate_would_under_bill_a_provider_that_sends_deltas() {
    // The hazard behind the naming problem in #667, stated as a number.
    //
    // A provider streaming 10 chunks of 100 output tokens each has produced 1000. Because `accumulate`
    // stores rather than adds, the counter reports 100 -- **a tenth of the usage** -- and the same request
    // sent as one non-streaming response would be billed correctly. Nothing reports an error.
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
        got.output_tokens, 100,
        "if this is 1000 the method has become a real accumulator, which is what #667 asks a decision about"
    );
}

/// The contract #667 asks for: the two write paths must not be distinguishable only by their names.
///
/// **Ignored, not deleted**, and failing today. It is deliberately not asserting one particular resolution,
/// because two are defensible and the choice is a design decision rather than a repair:
///
/// * rename `accumulate` to something that says "snapshot", leaving the behaviour and the Anthropic caller
///   untouched -- the cheap option, and correct for the only caller that exists;
/// * or make it sum, and give the Anthropic path the cumulative-total assignment it actually needs.
///
/// What the test rejects is the current state: a name and a doc comment that promise addition while the code
/// replaces, with a silent under-bill as the failure mode. It fails if either resolution is left undone by
/// checking the one property both share -- that a caller can tell which semantics it is getting.
#[ignore = "the contract for #667: `accumulate` must not promise addition while storing"]
#[test]
fn the_two_write_paths_are_not_distinguishable_only_by_name() {
    let c = UnifiedTokenCounter::new();
    c.accumulate(&usage(100, 100));
    c.accumulate(&usage(100, 100));

    let after_accumulate = c.get_usage();

    // Either semantics is acceptable, but the doc comment must match the behaviour. The comment on
    // `accumulate` says "incremental deltas", so under that contract this must be 200.
    let addition_semantics = after_accumulate.input_tokens == 200;

    // If it is not addition, then it is a snapshot, and a function documented as a snapshot must not be the
    // one whose name is `accumulate`.
    let documented_as_snapshot =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/counter.rs"))
            .map(|src| {
                let doc_start = src
                    .find("/// Accumulate usage from a partial chunk")
                    .unwrap_or(0);
                let doc_end = src[doc_start..].find("pub fn").unwrap_or(0) + doc_start;
                let doc = &src[doc_start..doc_end];
                doc.contains("snapshot")
                    || doc.contains("cumulative total")
                    || doc.contains("does not add")
            })
            .unwrap_or(false);

    assert!(
        addition_semantics || documented_as_snapshot,
        "`accumulate` stores rather than adds ({after_accumulate:?} after two 100-token events), while its \
         name and its doc comment say it accumulates deltas. Either the behaviour or the documentation has \
         to change; leaving both is what lets a delta-sending provider be under-billed silently"
    );
}
