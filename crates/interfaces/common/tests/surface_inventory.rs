#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the declaration guard asserts on parsed source and on known-good fixtures, and fails fast."
)]
//! Domain-declaration guard for `burncloud_common` (S1-D, extended by S1-E and S1-F).
//!
//! ## What this checks, precisely
//!
//! S1-A..S1-F moved, deleted or retired every domain type that used to live in this crate. The
//! invariant worth protecting is: **`burncloud_common` must not declare a domain type again.** The
//! next feature that needs a `User`, a price or an OpenAI DTO can reintroduce one in a single line,
//! and that regression is invisible in review if nobody knows the history.
//!
//! The guard parses the crate's own source for `pub struct` / `pub enum` / `pub type` / `pub fn` /
//! `pub const` / `pub trait` declarations (comments excluded) and fails when a name whose owner
//! lives elsewhere reappears.
//!
//! After S1-F the crate declares no domain type at all: `types.rs` is pure re-export, and what
//! remains are two Interfaces URL constants and the Platform-facing `CrudRepository` trait.
//!
//! ## What this does NOT check
//!
//! It is **not** an exact public-API snapshot. Three limits are deliberate, so the test's claim
//! matches its power:
//!
//! 1. It covers declarations, not re-export members. That the re-export blocks still exist is
//!    checked by `only_the_known_interfaces_items_remain`, but their contents are not enumerated.
//! 2. `common::*` still forwards the nanodollar helpers through `lib.rs`. Those names are **not**
//!    guarded here.
//! 3. It cannot see through a `#[path]` module or a macro-generated declaration.
//!
//! A full public-API snapshot would need re-export expansion; that is a different test with a
//! different name, and it is not what this file claims to be.

use std::fs;
use std::path::PathBuf;

/// The crate's source files. `types.rs` is where the domain types used to live.
const SRC_FILES: &[&str] = &[
    "src/lib.rs",
    "src/types.rs",
    "src/constants.rs",
    "src/repository.rs",
];

/// Item kinds the parser understands. Anything not listed is invisible to the guard.
const KINDS: &[&str] = &["struct", "enum", "type", "fn", "const", "trait"];

/// Names that left this crate, with the owner that must provide them instead.
///
/// Only names whose *declaration* is meaningful are listed: a helper such as `dollars_to_nano` is
/// not a declaration in this crate today, so listing it here would imply coverage the parser does
/// not have (limit 2 above).
const MOVED_OUT: &[(&str, &str)] = &[
    // Commerce contract, S1-A / S1-B.
    ("Price", "Commerce contract (S1-B)"),
    ("PriceInput", "Commerce contract (S1-B)"),
    ("TieredPrice", "Commerce contract (S1-B)"),
    ("TieredPriceInput", "Commerce contract (S1-B)"),
    ("FullPricing", "Commerce contract (S1-B)"),
    ("MultiCurrencyPrice", "Commerce contract (S1-B)"),
    ("ExchangeRate", "Commerce contract (S1-B)"),
    ("Currency", "Commerce contract (S1-B)"),
    ("PricingConfig", "Commerce contract (S1-B)"),
    ("ModelPricing", "Commerce contract (S1-B)"),
    ("CurrencyPricing", "Commerce contract (S1-B)"),
    ("TieredPriceConfig", "Commerce contract (S1-B)"),
    ("ValidationWarning", "Commerce contract (S1-B)"),
    ("ValidationError", "Commerce contract (S1-B)"),
    // Supply contract, S1-C.
    ("ChannelType", "Supply contract (S1-C)"),
    ("Channel", "Supply contract (S1-C)"),
    ("Ability", "Supply contract (S1-C)"),
    // Traffic contract, S1-E.
    ("TrafficColor", "Traffic contract (S1-E)"),
    ("OpenAIChatMessage", "Traffic contract (S1-E)"),
    ("OpenAIChatRequest", "Traffic contract (S1-E)"),
    ("OpenAIChatResponse", "Traffic contract (S1-E)"),
    ("OpenAIChatChoice", "Traffic contract (S1-E)"),
    (
        "RequestMapping",
        "Traffic; the live definition is traffic/router/src/adaptor/mapping.rs",
    ),
    (
        "ResponseMapping",
        "Traffic; the live definition is traffic/router/src/adaptor/mapping.rs",
    ),
    // Deleted in S1-D: no consumer, superseded by the Identity types.
    (
        "Recharge",
        "deleted in S1-D; superseded by Identity's UserRecharge",
    ),
    (
        "Token",
        "deleted in S1-D; superseded by UserApiKey / RouterToken",
    ),
    ("User", "deleted in S1-D; superseded by UserAccount"),
    // Deleted in S1-F: field-for-field duplicate of Supply's ChannelProtocolConfig, no consumer.
    (
        "ProtocolConfig",
        "deleted in S1-F; Supply's ChannelProtocolConfig is the owner (same fields, plus a method)",
    ),
    // Retired in S1-F: the legacy combination error, with no consumer anywhere.
    (
        "BurnCloudError",
        "deleted in S1-F; each owner returns its own error and the entry point maps it",
    ),
];

/// Items that legitimately remain, with the reason each one stays.
const EXPECTED_REMAINING: &[(&str, &str)] = &[
    (
        "pub const DEFAULT_PORT",
        "Interfaces URL constant; consumed by the CLI",
    ),
    (
        "pub const INTERNAL_PREFIX",
        "Interfaces URL constant; consumed by the router tests",
    ),
    (
        "pub trait CrudRepository",
        "Platform persistence abstraction; the templates and the token repository implement it",
    ),
];

fn read(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// `"pub struct User"`-style declaration names, with comment lines excluded so a documentation
/// example such as `/// pub struct UserRepository { .. }` is not mistaken for a declaration.
fn declarations(text: &str) -> Vec<String> {
    let mut items = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("//") || line.starts_with('*') {
            continue;
        }
        for kind in KINDS {
            let prefix = format!("pub {kind} ");
            if let Some(rest) = line.strip_prefix(&prefix) {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    items.push(format!("{prefix}{name}"));
                }
            }
        }
    }
    items.sort();
    items.dedup();
    items
}

#[test]
fn common_does_not_declare_domain_types() {
    let mut violations = Vec::new();
    for rel in SRC_FILES {
        let declared = declarations(&read(rel));
        for (name, owner) in MOVED_OUT {
            for kind in KINDS {
                let decl = format!("pub {kind} {name}");
                if declared.iter().any(|d| d == &decl) {
                    violations.push(format!("{rel} declares `{decl}` -- owner: {owner}"));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "burncloud_common must not declare domain types again:\n  {}\n\n\
         Move the item to its owning domain (or its contract crate) instead of adding it here.\n\
         Owners are recorded in #610 / #613 / #615 and docs/architecture-s1-task-contracts.md.",
        violations.join("\n  ")
    );
}

#[test]
fn only_the_known_interfaces_items_remain() {
    // The declarations that remain live in constants.rs and repository.rs. Anything new has to be
    // classified: either it belongs to Interfaces, or it belongs to a domain and must not be here.
    let mut declared: Vec<String> = SRC_FILES
        .iter()
        .flat_map(|f| declarations(&read(f)))
        .collect();
    declared.sort();
    declared.dedup();

    let mut expected: Vec<String> = EXPECTED_REMAINING
        .iter()
        .map(|(n, _)| n.to_string())
        .collect();
    expected.sort();
    expected.dedup();

    assert_eq!(
        declared, expected,
        "the declaration inventory of burncloud_common changed.\n\
         Either add the item to EXPECTED_REMAINING with the reason it stays, or move it to the\n\
         owning domain. Declaring a domain type here is what S1-A..S1-F removed."
    );

    // The re-exports are not declarations, so check them separately.
    let types = read("src/types.rs");
    for (needle, what) in [
        (
            "pub use burncloud_commerce_contracts::pricing::{",
            "Commerce pricing",
        ),
        ("pub use burncloud_supply_contracts::{", "Supply"),
        ("pub use burncloud_traffic_contracts::{", "Traffic"),
    ] {
        assert!(
            types.contains(needle),
            "the {what} re-export disappeared from common::types"
        );
    }
    assert!(
        read("src/lib.rs").contains("pub use burncloud_commerce_contracts::price_u64::{"),
        "the nanodollar re-export disappeared from the crate root"
    );
}

#[test]
fn types_module_is_re_export_only() {
    // The end state of S1: the module that used to hold every shared type now holds none. If a
    // declaration reappears here, the migration has started to unwind.
    let declared = declarations(&read("src/types.rs"));
    assert!(
        declared.is_empty(),
        "common::types must be re-export only after S1-F, found declarations: {declared:?}"
    );
}
