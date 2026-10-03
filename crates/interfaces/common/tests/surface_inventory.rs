#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Domain-declaration guard for `burncloud_common` (S1-D, #610).
//!
//! ## What this checks, precisely
//!
//! S1-A..S1-D moved or deleted the domain types that used to live in this crate. The invariant worth
//! protecting is: **`burncloud_common` must not declare a domain type again.** The next feature that
//! needs a `User` or a price can reintroduce one in a single line, and that regression is invisible
//! in review if nobody knows the history.
//!
//! The guard therefore parses the crate's own source for `pub struct` / `pub enum` / `pub type` /
//! `pub fn` / `pub const` / `pub trait` declarations (comments excluded) and fails when a name whose
//! owner lives elsewhere reappears.
//!
//! ## What this does NOT check
//!
//! It is **not** an exact public-API snapshot. Three limits are deliberate, so the test's claim
//! matches its power:
//!
//! 1. It covers declarations, not re-exports. The two contract re-export blocks (`pub use`) are
//!    checked to still exist by `contract_reexports_are_intact`, but their member lists are not
//!    enumerated.
//! 2. `common::*` still forwards the nanodollar helpers through `lib.rs`
//!    (`pub use burncloud_commerce_contracts::price_u64::{...}`) and through the `price_u64` module.
//!    Those names are **not** guarded here.
//! 3. It cannot see through a `#[path]` module or a macro-generated declaration.
//!
//! A full public-API snapshot would need support for every item kind plus re-export expansion; that
//! is a different test with a different name, and it is not what this file claims to be.

use std::fs;
use std::path::PathBuf;

/// The crate's source files. `types.rs` is where the domain types used to live.
const SRC_FILES: &[&str] = &[
    "src/lib.rs",
    "src/types.rs",
    "src/constants.rs",
    "src/error.rs",
    "src/repository.rs",
    "src/price_u64.rs",
    "src/pricing_config.rs",
];

/// Item kinds the parser understands. Anything not listed is invisible to the guard.
const KINDS: &[&str] = &["struct", "enum", "type", "fn", "const", "trait"];

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

/// Names that left this crate, with the owner that must provide them instead.
///
/// Only names whose *declaration* is meaningful are listed: a helper like `dollars_to_nano` is not a
/// declaration in this crate today, so listing it here would imply coverage the parser does not have
/// (limit 2 above).
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
];

/// The inventory of `src/types.rs` that is expected to remain, with each item's owner.
///
/// `ChannelType` / `Channel` / `Ability` are absent on purpose: since S1-C they are re-exports from
/// the Supply contract, not declarations in this file.
const EXPECTED_TYPES_RS: &[(&str, &str)] = &[
    ("pub enum TrafficColor", "Interfaces -> Traffic (S1-E)"),
    (
        "pub struct OpenAIChatChoice",
        "Interfaces -> Traffic (S1-E)",
    ),
    (
        "pub struct OpenAIChatMessage",
        "Interfaces -> Traffic (S1-E)",
    ),
    (
        "pub struct OpenAIChatRequest",
        "Interfaces -> Traffic (S1-E)",
    ),
    (
        "pub struct OpenAIChatResponse",
        "Interfaces -> Traffic (S1-E)",
    ),
    (
        "pub struct ProtocolConfig",
        "Interfaces -> Supply/Traffic (S1-F)",
    ),
    ("pub struct RequestMapping", "Interfaces -> Traffic (S1-E)"),
    ("pub struct ResponseMapping", "Interfaces -> Traffic (S1-E)"),
    // `impl TrafficColor { pub fn as_char(&self) }` -- the parser sees the method.
    ("pub fn as_char", "Traffic, moved with TrafficColor (S1-E)"),
];

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
         Owners are recorded in #610 and docs/architecture-s1-task-contracts.md.",
        violations.join("\n  ")
    );
}

#[test]
fn types_rs_inventory_is_unchanged() {
    let mut declared = declarations(&read("src/types.rs"));
    // The two contract re-export blocks are `pub use`, not declarations; record their presence so a
    // new declaration cannot hide behind them.
    let text = read("src/types.rs");
    if text.contains("pub use burncloud_commerce_contracts::pricing::{") {
        declared.push("pub use::commerce".to_string());
    }
    if text.contains("pub use burncloud_supply_contracts::{") {
        declared.push("pub use::supply".to_string());
    }
    declared.sort();
    declared.dedup();

    let mut expected: Vec<String> = EXPECTED_TYPES_RS
        .iter()
        .map(|(n, _)| n.to_string())
        .chain([
            "pub use::commerce".to_string(),
            "pub use::supply".to_string(),
        ])
        .collect();
    expected.sort();
    expected.dedup();

    assert_eq!(
        declared, expected,
        "the declaration inventory of burncloud_common::types changed.\n\
         Either add the item to EXPECTED_TYPES_RS with its owner, or move it to the owning domain.\n\
         Listing it here is only correct if it really belongs to the Interfaces layer."
    );

    assert!(
        EXPECTED_TYPES_RS.len() >= 8,
        "EXPECTED_TYPES_RS shrank unexpectedly; the inventory was trimmed without a review"
    );
}

#[test]
fn contract_reexports_are_intact() {
    // The compatibility layer is the only reason the moved domain types are still reachable through
    // common; if a re-export disappears, existing consumers break.
    let text = read("src/types.rs");
    assert!(
        text.contains("pub use burncloud_commerce_contracts::pricing::{"),
        "the Commerce pricing re-export disappeared from common::types"
    );
    assert!(
        text.contains("pub use burncloud_supply_contracts::{"),
        "the Supply re-export disappeared from common::types"
    );
    let lib = read("src/lib.rs");
    assert!(
        lib.contains("pub use burncloud_commerce_contracts::price_u64::{"),
        "the nanodollar re-export disappeared from the crate root"
    );
}
