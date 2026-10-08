#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    reason = "Test-only file: the assertions are the test and the fixtures are JSON of unknown shape."
)]
//! Version handling and tier boundaries for the Commerce pricing contract (#633, plan section 5 item 15).
//!
//! The plan's remaining items for this crate are "旧版本兼容" and "未知版本处理" -- backward compatibility and
//! what happens to a version the code does not recognise. `pricing_fixtures.rs` and `money_boundaries.rs`
//! already cover the fixture values, the nanodollar scale, rounding and overflow, so this file does not repeat
//! them; it covers **which parser a document is sent to**, and the tier rules.
//!
//! ## What decides the path
//!
//! ```text
//! pricing/mod.rs:983-988
//!   let version = match value["version"].as_str() {
//!       Some(v) => v.to_string(),
//!       None => "1.0".to_string(),      // documented fallback
//!   };
//!   let is_new_format = version_major(&version) >= 7;
//! ```
//!
//! and `version_major` is `split('.').next().parse().unwrap_or(1)`. So **every** string reaches a parser: a
//! version that cannot be read as a number becomes **1**, which means the v1 parser. There is no "unsupported
//! version" branch. These tests measure what that costs, rather than assuming it is harmless.
//!
//! ## A defect that is documented, not asserted as a contract
//!
//! `pricing_fixtures.rs:215` records that a v7 document's price blocks do not survive `to_json()` followed by
//! re-parsing, because `to_json()` omits `version` and the output therefore re-enters the v1 path. That test
//! says "documented gap ... Until that is fixed, a v7 document must not be round-tripped". This file relies on
//! it the same way: the version tests below parse a document **once**, and the round-trip defect is referenced
//! rather than re-frozen.

use burncloud_commerce_contracts::pricing::PricingConfig;

/// A v7-shaped document (flat, currency-first) with the given version string.
fn v7_doc(version: &str) -> String {
    format!(
        r#"{{"version":"{version}","updated_at":"2026-03-30T00:00:00Z","source":"test",
             "models":{{"m":{{"USD":{{"text":{{"in":3.0,"out":15.0}}}}}}}}}}"#
    )
}

/// A v1-shaped document (nested under `pricing`) with the given version string.
fn v1_doc(version: &str) -> String {
    format!(
        r#"{{"version":"{version}","updated_at":"2026-03-30T00:00:00Z","source":"test",
             "models":{{"m":{{"pricing":{{"USD":{{"input_price":3.0,"output_price":15.0}}}}}}}}}}"#
    )
}

/// The same models, without any `version` field at all.
fn unversioned_v7_doc() -> String {
    r#"{"updated_at":"2026-03-30T00:00:00Z","source":"test",
        "models":{"m":{"USD":{"text":{"in":3.0,"out":15.0}}}}}"#
        .to_string()
}

/// `3.0` USD per million tokens in nanodollars.
const THREE_USD: i64 = 3_000_000_000;

// -------------------------------------------------------------------------------------------
// 旧版本兼容
// -------------------------------------------------------------------------------------------

#[test]
fn a_legacy_version_still_parses_through_the_v1_path() {
    // The compatibility half of the plan's item. A document claiming 1.0 must keep working, and its numbers
    // must be the ones the v1 shape describes -- not the v7 ones, which would mean the version was ignored.
    for version in ["1.0", "1.1", "6.9"] {
        let config = PricingConfig::from_json(&v1_doc(version))
            .unwrap_or_else(|e| panic!("{version} must parse: {e}"));
        let pricing = config
            .get_pricing("m", "USD")
            .unwrap_or_else(|| panic!("{version}: pricing must exist"));

        assert_eq!(
            pricing.input_price, THREE_USD,
            "{version}: the nested v1 shape must be read"
        );
        assert_eq!(
            config.version, version,
            "{version}: and the version string is carried through rather than replaced"
        );
    }
}

#[test]
fn a_version_without_a_minor_part_is_dispatched_by_its_major_number() {
    // `version_major` splits on '.' and parses the first part, so "7" and "7.0" are the same major version and
    // must take the same path. A document written by hand or by a tool that trims the minor part would
    // otherwise be mis-read.
    let seven =
        PricingConfig::from_json(&v7_doc("7")).unwrap_or_else(|e| panic!("7 must parse: {e}"));
    assert_eq!(
        seven.get_pricing("m", "USD").unwrap().input_price,
        THREE_USD,
        "\"7\" is major version 7, so the flat v7 shape is read"
    );

    let nested =
        PricingConfig::from_json(&v1_doc("6")).unwrap_or_else(|e| panic!("6 must parse: {e}"));
    assert_eq!(
        nested.get_pricing("m", "USD").unwrap().input_price,
        THREE_USD,
        "\"6\" is below 7, so the nested v1 shape is read"
    );

    // The two shapes really are different parsers, and since #606 the **body** decides which one runs: the
    // same flat v7 body under a pre-7 version is still read by the v7 parser, because a body that carries
    // `text` blocks is not something the v1 parser can describe. Mislabelled input is recovered rather than
    // silently emptied.
    let flat_body_under_six = PricingConfig::from_json(&v7_doc("6.0"))
        .unwrap_or_else(|e| panic!("a flat body under 6.0 must parse: {e}"));
    let found = flat_body_under_six.get_pricing("m", "USD").is_some();
    println!("a flat body under version 6.0 yields pricing: {found}");
    assert!(
        found,
        "the body's `text` block identifies the flat shape, so the price is read even though the version \
         string says 6.0 -- the layout marker is the fact, the version only breaks a tie (#606)"
    );

    // A nested v1 body with a v7 version is the mirror case, and it is the one `to_json` produces: read as
    // v1, so nothing is lost.
    let nested_body_under_seven = PricingConfig::from_json(&v1_doc("7.0"))
        .unwrap_or_else(|e| panic!("a nested body under 7.0 must parse: {e}"));
    assert!(
        nested_body_under_seven.get_pricing("m", "USD").is_some(),
        "a nested body must be read as nested whatever the version says, or `to_json` output would lose its \
         prices again"
    );
}

// -------------------------------------------------------------------------------------------
// 未知版本处理
// -------------------------------------------------------------------------------------------

#[test]
fn an_unreadable_version_still_reads_the_body_when_the_layout_is_recognisable() {
    // **What "unknown version" does now (#606).** `version_major` is
    // `split('.').next().parse().unwrap_or(1)`, so a version that is not a number becomes **1**. But the
    // layout is checked first, so an unreadable version is no longer a way to lose the prices: the body's own
    // shape decides, and only a body carrying no marker falls back to the v1 reading.
    //
    // Both halves matter. The nested v1 shape keeps parsing, as before.
    for version in ["abc", "", "v7.0", " 7.0", "seven"] {
        let result = PricingConfig::from_json(&v1_doc(version));
        match result {
            Ok(config) => {
                let found = config.get_pricing("m", "USD").is_some();
                println!("v1 body, version {version:?} -> parsed, pricing found: {found}");
                assert!(
                    found,
                    "a nested v1 body must be read whatever the version string says: {version:?}"
                );
            }
            Err(e) => println!("v1 body, version {version:?} -> rejected: {e}"),
        }
    }

    // And the flat v7 body is now recovered rather than silently emptied. This is the #606 repair seen from
    // the other side: a mislabelled document used to lose every price with no error anywhere.
    for version in ["abc", "", "v7.0"] {
        let config = PricingConfig::from_json(&v7_doc(version))
            .unwrap_or_else(|e| panic!("{version}: the document still parses: {e}"));
        let found = config.get_pricing("m", "USD").is_some();
        println!("a flat v7 body under {version:?} -> pricing found: {found}");
        assert!(
            found,
            "version {version:?} is not readable as 7, but the body's `text` block identifies the flat shape, \
             so the price must be found rather than silently absent"
        );
        assert_eq!(
            config.get_pricing("m", "USD").unwrap().input_price,
            THREE_USD,
            "and with the right value, not a placeholder"
        );
    }
}

#[test]
fn a_version_above_the_known_range_is_treated_as_the_newest_format() {
    // The other direction: `>= 7` means an unknown **future** major version takes the v7 path rather than being
    // rejected. That is a deliberate forward-compatibility choice, and this pins it so it is a decision rather
    // than an accident -- a document from a later release parses with the newest parser available instead of
    // failing outright.
    for version in ["8.0", "9.1", "10.0", "99.0"] {
        let config = PricingConfig::from_json(&v7_doc(version))
            .unwrap_or_else(|e| panic!("{version} must parse: {e}"));
        assert_eq!(
            config.get_pricing("m", "USD").unwrap().input_price,
            THREE_USD,
            "{version} is >= 7, so the flat shape is read by the newest parser"
        );
        assert_eq!(
            config.version, version,
            "and the version string is preserved verbatim"
        );
    }

    // The boundary itself, from both sides, because `>=` and `>` differ exactly here.
    let six = PricingConfig::from_json(&v1_doc("6.9")).unwrap();
    let seven = PricingConfig::from_json(&v7_doc("7.0")).unwrap();
    assert_eq!(six.version, "6.9");
    assert_eq!(seven.version, "7.0");
    assert!(
        six.get_pricing("m", "USD").is_some() && seven.get_pricing("m", "USD").is_some(),
        "6.9 is read as v1 and 7.0 as v7; both find the price, but from different shapes"
    );
}

#[test]
fn a_document_with_no_version_still_has_its_body_read() {
    // The documented fallback, restated here because the version tests above lean on it: absence is not the
    // same as a bad value, and both read as 1.0. Since #606 the layout is checked first, so an unversioned
    // **flat** body is still read by the v7 parser instead of being handed to one that cannot describe it.
    let flat = PricingConfig::from_json(&unversioned_v7_doc()).unwrap();
    assert_eq!(flat.version, "1.0", "a missing version reads as 1.0");
    assert_eq!(
        flat.get_pricing("m", "USD").unwrap().input_price,
        THREE_USD,
        "and the flat body's price is found: an unversioned document must not silently lose its prices just \
         because the version string is absent"
    );

    // An unversioned **nested** body keeps the v1 reading, which is the original documented fallback.
    let nested = r#"{"updated_at":"2026-03-29T00:00:00Z","source":"t",
                     "models":{"m":{"pricing":{"USD":{"input_price":3.0,"output_price":4.0}}}}}"#;
    let nested = PricingConfig::from_json(nested).unwrap();
    assert_eq!(nested.version, "1.0");
    assert_eq!(
        nested.get_pricing("m", "USD").unwrap().input_price,
        THREE_USD,
        "and a nested body without a version is read as v1, as documented"
    );
}

// -------------------------------------------------------------------------------------------
// tier rules
// -------------------------------------------------------------------------------------------

/// A tiered document in the shape the contract reads: a v7 currency block with a `tiered` array.
///
/// The first version of this file used a v1-nested `tiers` array on the `pricing` block, and
/// `get_tiered_pricing` returned `None` for every one of the four tests below -- so the shape, not the rule,
/// was what failed. `fixtures/pricing/tiered.json` shows the real shape: tiers under the currency block as
/// `tiered`, with `in`/`out` rather than `input_price`/`output_price`.
fn tiered_doc() -> String {
    r#"{"version":"7.0","updated_at":"2026-03-30T00:00:00Z","source":"test",
        "models":{"qwen":{"USD":{"text":{"in":1.0,"out":2.0},
          "tiered":[
            {"tier_start":0,"tier_end":32000,"in":1.0,"out":2.0},
            {"tier_start":32000,"tier_end":128000,"in":2.0,"out":4.0},
            {"tier_start":128000,"in":3.0,"out":6.0}
          ]}}}}"#
        .to_string()
}

#[test]
fn tiers_are_contiguous_and_the_last_one_is_open_ended() {
    // The plan's "tier 起止" item. Two invariants that make a tier table usable: each tier begins where the
    // previous one ended (so no token count falls in a gap), and the final tier has no end (so no token count
    // falls off the top). A gap would make a request unpriceable; an end on the last tier would do the same.
    let config = PricingConfig::from_json(&tiered_doc()).unwrap();
    let tiers = config
        .get_tiered_pricing("qwen", "USD")
        .unwrap_or_else(|| panic!("the tiers must be read"));

    assert_eq!(tiers.len(), 3, "all three tiers are kept");
    for pair in tiers.windows(2) {
        let (earlier, later) = (&pair[0], &pair[1]);
        assert_eq!(
            earlier.tier_end,
            Some(later.tier_start),
            "tier starting at {} must end where the next begins, or a token count falls in a gap",
            later.tier_start
        );
    }
    assert_eq!(tiers[0].tier_start, 0, "the first tier starts at zero");
    assert_eq!(
        tiers.last().unwrap().tier_end,
        None,
        "the last tier is open-ended, so any token count is priceable"
    );

    // The prices are distinct per tier, so a lookup that ignored the tier would be visible rather than looking
    // like a rounding difference.
    let inputs: Vec<i64> = tiers.iter().map(|t| t.input_price).collect();
    println!("tier input prices: {inputs:?}");
    assert_eq!(inputs, vec![1_000_000_000, 2_000_000_000, 3_000_000_000]);
}

#[test]
fn every_token_count_lands_in_exactly_one_tier() {
    // The property the contiguity above is for, checked at the boundaries rather than in the middle: the first
    // token of a tier, the last token before it ends, and the boundaries themselves. A tier table is chosen by
    // comparing a token count against `tier_start` and `tier_end`, so an off-by-one at a boundary is the failure
    // mode -- and it is the one a mid-range test cannot see.
    let config = PricingConfig::from_json(&tiered_doc()).unwrap();
    let tiers = config.get_tiered_pricing("qwen", "USD").unwrap();

    // The tier a count belongs to, by the convention `tier_end` is exclusive: a count belongs to the first tier
    // whose `tier_start` it reaches.
    let tier_for = |tokens: i64| -> Option<usize> {
        tiers
            .iter()
            .position(|t| tokens >= t.tier_start && t.tier_end.map(|e| tokens < e).unwrap_or(true))
    };

    for tokens in [
        0, 1, 31_999, 32_000, 32_001, 127_999, 128_000, 128_001, 1_000_000,
    ] {
        let idx = tier_for(tokens);
        println!("{tokens} tokens -> tier {idx:?}");
        assert!(
            idx.is_some(),
            "{tokens} tokens must belong to exactly one tier; None means a gap in the table"
        );
    }

    // And the boundaries themselves, asserted as figures rather than as `is_some`, so the convention is pinned:
    // 32_000 is the first token of the second tier, not the last of the first.
    assert_eq!(tier_for(31_999), Some(0), "the last token of tier 0");
    assert_eq!(tier_for(32_000), Some(1), "the first token of tier 1");
    assert_eq!(tier_for(127_999), Some(1), "the last token of tier 1");
    assert_eq!(tier_for(128_000), Some(2), "the first token of tier 2");
    assert_eq!(
        tier_for(i64::MAX),
        Some(2),
        "and the open-ended tier covers the largest representable count"
    );
}

#[test]
fn overlapping_tiers_are_stored_as_written_and_the_first_match_wins() {
    // The plan's "重叠" item. The fixture above is well-formed; this one is **not**, and the question is what the
    // contract does with it rather than what it should do. The document keeps both tiers and their order, so a
    // caller resolving a token count by scanning for the first match gets the **first** tier -- the narrower one
    // -- rather than the later, wider one that also covers the count.
    //
    // Recorded rather than fixed: the plan says to test the interfaces as they are. What matters is that the
    // overlap is **preserved and ordered** rather than silently collapsed, so a caller can detect it.
    let json = r#"{"version":"7.0","updated_at":"2026-03-30T00:00:00Z","source":"test",
        "models":{"overlap":{"USD":{"text":{"in":1.0,"out":2.0},
          "tiered":[
            {"tier_start":0,"tier_end":100000,"in":1.0,"out":2.0},
            {"tier_start":50000,"tier_end":200000,"in":9.0,"out":9.0}
          ]}}}}"#;
    let config = PricingConfig::from_json(json).unwrap();
    let tiers = config.get_tiered_pricing("overlap", "USD").unwrap();

    assert_eq!(
        tiers.len(),
        2,
        "both overlapping tiers are kept, not merged or dropped"
    );
    assert_eq!(tiers[0].tier_start, 0);
    assert_eq!(tiers[1].tier_start, 50_000);
    println!(
        "overlapping tiers: {:?}",
        tiers
            .iter()
            .map(|t| (t.tier_start, t.tier_end, t.input_price))
            .collect::<Vec<_>>()
    );

    // 60_000 is inside both. A first-match scan returns the first, whose price is 1 nano; the second tier's 9
    // nano would be a different charge for the same request.
    let tokens = 60_000;
    let first_match = tiers
        .iter()
        .position(|t| tokens >= t.tier_start && t.tier_end.map(|e| tokens < e).unwrap_or(true))
        .expect("an overlapping table still matches");
    assert_eq!(first_match, 0, "the first match is the earlier tier");
    assert_eq!(
        tiers[first_match].input_price, 1_000_000_000,
        "so an overlapping document charges the FIRST tier's price, which is the lower one here"
    );
    // And the version is dispatched **verbatim**, with no case folding or trimming: a numeric prefix is what
    // `version_major` reads, so `"7.0"` works and `"V7.0"` does not. Worth pinning because a lowercasing or
    // trimming step added later would be invisible in every other test here (a mutation that lowercased the
    // version survived the whole suite until this case was added) while quietly changing which documents are
    // readable.
    for version in ["7.0", "7.0.1", "07.0"] {
        let config = PricingConfig::from_json(&v7_doc(version))
            .unwrap_or_else(|e| panic!("{version} must parse: {e}"));
        assert_eq!(
            config.get_pricing("m", "USD").unwrap().input_price,
            THREE_USD,
            "{version:?} has a numeric major part, so the v7 path is taken"
        );
    }
    // And the body, not the version string, is what makes a document readable. `"V7.0"` is not numeric, so it
    // is no longer a way to strip the prices: the flat `text` block identifies the shape and the dollar values
    // are read. #606 made the layout marker authoritative precisely because a version string was able to
    // empty a perfectly well-formed body with no error.
    let uppercase = PricingConfig::from_json(&v7_doc("V7.0")).unwrap();
    assert_eq!(
        uppercase.get_pricing("m", "USD").unwrap().input_price,
        THREE_USD,
        "\"V7.0\" is not numeric, but the flat body is still recognised from its `text` block"
    );
    assert_eq!(
        uppercase.version, "V7.0",
        "and the version string is carried verbatim rather than normalised"
    );
}

#[test]
fn a_tier_list_that_starts_above_zero_leaves_a_gap_which_is_visible() {
    // The complementary defect to the overlap: a table whose first tier starts above zero has token counts that
    // belong to no tier. Nothing rejects it, so the gap reaches the caller -- and this test exists so that "no
    // tier matched" is a known possibility rather than an assumption that it cannot happen.
    let json = r#"{"version":"7.0","updated_at":"2026-03-30T00:00:00Z","source":"test",
        "models":{"gap":{"USD":{"text":{"in":1.0,"out":2.0},
          "tiered":[{"tier_start":100,"tier_end":200,"in":1.0,"out":2.0}]}}}}"#;
    let config = PricingConfig::from_json(json).unwrap();
    let tiers = config.get_tiered_pricing("gap", "USD").unwrap();

    assert_eq!(tiers[0].tier_start, 100, "the gap is stored as written");
    let tier_for = |tokens: i64| -> Option<usize> {
        tiers
            .iter()
            .position(|t| tokens >= t.tier_start && t.tier_end.map(|e| tokens < e).unwrap_or(true))
    };

    println!(
        "0 tokens -> {:?}, 100 -> {:?}, 200 -> {:?}",
        tier_for(0),
        tier_for(100),
        tier_for(200)
    );
    assert_eq!(
        tier_for(0),
        None,
        "a count below the first tier matches nothing"
    );
    assert_eq!(
        tier_for(100),
        Some(0),
        "and the first tier covers its own start"
    );
    assert_eq!(
        tier_for(200),
        None,
        "a count at or above the only tier's end matches nothing either, because no tier is open-ended"
    );
}
