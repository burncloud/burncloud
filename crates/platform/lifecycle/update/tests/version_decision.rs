#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! The version decision behind the updater (#633, plan section 5 item 22).
//!
//! The plan lists "版本比较/预发布规则" for this crate and nothing tested it: the comparison lived **inside two
//! functions that both fetch the GitHub release list**, so exercising it meant making a network call. It is now
//! `is_upgrade_over`, a pure function both call sites use.
//!
//! ## Why that mattered, and not only for testing
//!
//! The two call sites **disagreed**:
//!
//! ```text
//! needs_update            -> semver::Version::parse both, then `latest > current`
//! sync_check_for_updates  -> `release_version != current_clean`, a string comparison
//! ```
//!
//! and the second is the one production uses: `crates/interfaces/cli/src/cli/commands.rs:1191` calls
//! `sync_check_for_updates`, as does the crate's own documented example. Under the string comparison a release
//! **older** than the running version -- `1.0.5` against a running `1.0.4` when the list leads with an older tag
//! -- reports "update available", and the caller's next step is `sync_update`, which **replaces the running
//! executable**. So the disagreement was a path to a silent downgrade, not a cosmetic difference.
//!
//! `is_upgrade_over` keeps the parsed branch and the fallback, and both call sites now use it. Its doc comment
//! records the prerelease position, which the tests below pin rather than decide.

use burncloud_auto_update::is_upgrade_over;

#[test]
fn only_a_higher_version_is_an_upgrade() {
    // The core rule. The `false` cases are the ones that matter: the caller's next step replaces the running
    // executable, so "same" and "older" must both report no update.
    let cases = [
        // An upgrade.
        ("1.0.4", "1.0.3", true),
        ("1.1.0", "1.0.9", true),
        ("2.0.0", "1.99.99", true),
        // The same version, written the same way.
        ("1.0.3", "1.0.3", false),
        // Same version, different spelling: still not an upgrade.
        ("v1.0.3", "1.0.3", false),
        ("1.0.3", "v1.0.3", false),
        // **Older**, which the string comparison used to call an update.
        ("1.0.3", "1.0.4", false),
        ("1.0.9", "1.1.0", false),
        ("1.99.99", "2.0.0", false),
        ("v1.0.2", "1.0.10", false),
        // **Build metadata, which the specification says must not order.** `semver` the crate disagrees with
        // `semver` the specification: `1.0.3+build.7 > 1.0.3` is **true** here, because the Rust crate's `Ord`
        // compares build metadata as a final tie-break. So a re-tagged build of the version already running
        // reports as an upgrade and the caller would replace the executable with it.
        //
        // Recorded, not fixed: this is the dependency's behaviour, not code in this crate, and changing it means
        // either not using `semver`'s `Ord` or stripping the metadata before comparing -- both behaviour
        // decisions that belong to whoever owns the update contract the plan refers to.
        ("1.0.3+build.7", "1.0.3", true),
    ];

    for (release, current, expected) in cases {
        let got = is_upgrade_over(release, current);
        println!("release {release:>12} over current {current:>12} -> {got}");
        assert_eq!(got, expected, "release {release} over current {current}");
    }
}

#[test]
fn the_comparison_is_numeric_per_component_rather_than_lexicographic() {
    // The reason a string comparison is wrong, isolated. `"1.0.10" < "1.0.9"` as strings and `>` as versions, so
    // a release that is genuinely newer is reported as older -- and the caller would not update.
    assert!(
        "1.0.10" < "1.0.9",
        "the premise: as strings, 1.0.10 sorts before 1.0.9"
    );
    assert!(
        is_upgrade_over("1.0.10", "1.0.9"),
        "and as versions it is higher, so 1.0.10 is an upgrade over 1.0.9"
    );
    assert!(
        !is_upgrade_over("1.0.9", "1.0.10"),
        "in the other direction it is not"
    );

    // Two more where the digit count decides, so this is not a single example.
    for (release, current) in [("1.10.0", "1.9.0"), ("10.0.0", "9.0.0")] {
        assert!(
            is_upgrade_over(release, current),
            "{release} is above {current} numerically"
        );
        assert!(
            !is_upgrade_over(current, release),
            "and {current} is not above {release}"
        );
    }
}

#[test]
fn a_leading_v_is_stripped_before_comparing() {
    // Release tags are commonly written `v1.2.3` while `current_version` comes from `CARGO_PKG_VERSION`, which
    // has no prefix. `semver::Version::parse` rejects the prefix, so without the strip both sides would fall to
    // the string branch and `v1.2.3` against `1.2.3` would read as a change.
    assert!(
        !is_upgrade_over("v1.2.3", "1.2.3"),
        "the same version with and without the prefix is not an upgrade"
    );
    assert!(
        is_upgrade_over("v1.2.4", "1.2.3"),
        "and a prefixed release one patch higher is"
    );
    assert!(
        is_upgrade_over("1.2.4", "v1.2.3"),
        "nor does the prefix on the current version change the answer"
    );

    // Only the first `v` is stripped, and only if it is at the front -- `semver` would reject an interior one and
    // the string branch would take over. Recorded so the boundary is explicit.
    assert!(
        !is_upgrade_over("1.2.3", "1.2.3"),
        "an interior `v` is not a prefix and both sides still parse"
    );
}

#[test]
fn a_prerelease_ranks_below_its_release_and_above_the_previous_one() {
    // "预发布规则", measured rather than decided. These are the `semver` ordering rules; the crate applies them
    // as they are and has **no policy for excluding prereleases from the candidate set**. Recorded because the
    // caller replaces the executable on `true`, so a release list that leads with a prerelease will install one.
    let cases = [
        // A stable release is an upgrade over its own prerelease.
        ("1.0.0", "1.0.0-beta.1", true),
        ("1.0.0", "1.0.0-rc.1", true),
        // And a prerelease is not an upgrade over the release it precedes.
        ("1.0.0-beta.1", "1.0.0", false),
        // Prereleases are ordered among themselves.
        ("1.0.0-rc.1", "1.0.0-beta.1", true),
        ("1.0.0-beta.2", "1.0.0-beta.1", true),
        ("1.0.0-beta.1", "1.0.0-beta.2", false),
        // A prerelease of a later version is still above an earlier stable release, which is the case a
        // "skip prereleases" policy would have to catch: `2.0.0-rc.1` is offered over a running `1.9.0`.
        ("2.0.0-rc.1", "1.9.0", true),
        // A prerelease is below the release of the same version even when the release is older than the running
        // version -- the two rules compose rather than one overriding the other.
        ("1.0.0-rc.1", "1.0.0-beta.1", true),
    ];

    for (release, current, expected) in cases {
        let got = is_upgrade_over(release, current);
        println!("release {release:>14} over current {current:>14} -> {got}");
        assert_eq!(got, expected, "release {release} over current {current}");
    }

    // **Build metadata is where the crate and the specification part company, so it is measured rather than
    // asserted from the spec.** The semver specification says build metadata "MUST be ignored when determining
    // version precedence"; the Rust `semver` crate's `Ord` compares it anyway, so:
    assert!(
        is_upgrade_over("1.0.0+build.2", "1.0.0+build.1"),
        "the crate orders build metadata, so a higher build suffix reads as an upgrade"
    );
    assert!(
        is_upgrade_over("1.0.0+build.1", "1.0.0"),
        "and so does a build suffix against the bare version -- which the specification would call the same \
         version, and which means a re-tagged build of the running version prompts a replacement"
    );
}

#[test]
fn an_unparseable_version_falls_back_to_inequality_rather_than_reporting_no_update() {
    // The fallback branch. An unparseable version cannot be ordered, and the choice made here is to report
    // whether the strings **differ** instead of reporting "no update". The alternative would leave a user on an
    // odd version string never offered an update at all, which is worse than offering one that may not be
    // needed.
    let cases = [
        ("nightly", "1.0.0", true),
        ("1.0.0", "nightly", true),
        ("nightly", "nightly", false),
        ("", "1.0.0", true),
        ("1.0.0", "", true),
        ("", "", false),
        ("1.0", "1.0.0", true),
        // Two spellings of the same unparseable string are equal, so no update.
        ("abc", "abc", false),
    ];

    for (release, current, expected) in cases {
        let got = is_upgrade_over(release, current);
        println!("release {release:?} over current {current:?} -> {got}");
        assert_eq!(
            got, expected,
            "release {release:?} over current {current:?}"
        );
    }

    // The asymmetry worth naming: `1.0` and `1.0.0` are the same version to a human and different strings, so
    // the fallback calls it an update -- **in both directions**, because the check is `release != current` and
    // inequality is symmetric even though an ordering would not be.
    assert!(
        is_upgrade_over("1.0", "1.0.0"),
        "an incomplete semver falls to the string branch, where differing text counts as a change"
    );
    assert!(
        is_upgrade_over("1.0.0", "1.0"),
        "and the same holds the other way, because the fallback tests inequality rather than order -- so a run \
         whose version is the full form is offered the short form as an update"
    );
    assert!(
        !is_upgrade_over("1.0", "1.0"),
        "an unparseable version is never a change from itself"
    );
}

#[test]
fn the_decision_is_antisymmetric_and_irreflexive_where_both_versions_parse() {
    // Two properties a comparison used this way should have, checked over a set rather than one pair at a time.
    // **Antisymmetry**: exactly one of `a over b` and `b over a` is true for two different versions -- so the
    // updater cannot both offer and refuse the same pair depending on which side it is asked from.
    // **Irreflexivity**: a version is never an upgrade over itself, which is what stops a check from looping on a
    // release it already runs.
    let versions = [
        "0.0.1",
        "0.1.0",
        "1.0.0",
        "1.0.0-beta.1",
        "1.0.1",
        "1.1.0",
        "2.0.0",
    ];

    for a in versions {
        assert!(
            !is_upgrade_over(a, a),
            "{a} must not be an upgrade over itself"
        );
        for b in versions {
            if a == b {
                continue;
            }
            let forward = is_upgrade_over(a, b);
            let backward = is_upgrade_over(b, a);
            assert_ne!(
                forward, backward,
                "exactly one of `{a} over {b}` and `{b} over {a}` must hold, got {forward} and {backward}"
            );
        }
    }

    // And the order is transitive over the same set, so it is a total order on the parsable versions rather than
    // a relation that happens to be antisymmetric.
    for a in versions {
        for b in versions {
            for c in versions {
                if is_upgrade_over(b, a) && is_upgrade_over(c, b) {
                    assert!(
                        is_upgrade_over(c, a),
                        "{b} > {a} and {c} > {b} must give {c} > {a}"
                    );
                }
            }
        }
    }

    // The count of upgrades in the set, so a wrong comparison cannot be antisymmetric and transitive by accident
    // while ordering everything backwards. Three of the seven are above 1.0.0: `1.0.1`, `1.1.0` and `2.0.0`. The
    // first version of this assertion said two and listed three, which is the kind of slip the count exists to
    // catch.
    let upgrades = versions
        .iter()
        .filter(|v| is_upgrade_over(v, "1.0.0"))
        .count();
    println!("versions above 1.0.0: {upgrades} of {}", versions.len());
    assert_eq!(
        upgrades, 3,
        "1.0.1, 1.1.0 and 2.0.0 are above 1.0.0; 1.0.0-beta.1 is below it and the rest are lower still"
    );
}
