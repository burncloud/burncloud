#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Surface inventory guard for `burncloud_common` (S1-D, #610).
//!
//! S1-A/B/C/D have been moving domain types out of the Interfaces layer. That work is easy to undo
//! by accident: the next feature that needs a `User` or a price "just adds it to common" again, and
//! the migration silently regresses. This guard makes that visible at test time by pinning the exact
//! public inventory of the crate and naming who owns each item.
//!
//! It reads the source instead of using reflection because Rust cannot assert the absence of a type
//! at compile time; the previous three subtasks already use this style for cross-module contracts
//! (see `contract_purity.rs` in the Commerce and Supply contract crates).

use std::fs;
use std::path::PathBuf;

/// Directories that used to host domain types and must not grow new ones.
const SRC_FILES: &[&str] = &[
    "src/lib.rs",
    "src/types.rs",
    "src/constants.rs",
    "src/error.rs",
    "src/repository.rs",
];

fn read(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Only `pub struct`/`pub enum`/`pub type` declarations plus the re-export blocks, so the guard
/// does not depend on formatting or comments.
fn declared_items(text: &str) -> Vec<String> {
    let mut items = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        for prefix in ["pub struct ", "pub enum ", "pub type "] {
            if let Some(rest) = line.strip_prefix(prefix) {
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

/// Items that left `common` in S1-A..S1-D and must not come back.
const MOVED_OUT: &[(&str, &str)] = &[
    ("dollars_to_nano", "Commerce contract (S1-A)"),
    ("Price", "Commerce contract (S1-B)"),
    ("PricingConfig", "Commerce contract (S1-B)"),
    ("Currency", "Commerce contract (S1-B)"),
    ("ChannelType", "Supply contract (S1-C)"),
    ("Channel", "Supply contract (S1-C)"),
    ("Ability", "Supply contract (S1-C)"),
    (
        "Recharge",
        "deleted in S1-D: no consumer, duplicated by Identity's UserRecharge",
    ),
    (
        "Token",
        "deleted in S1-D: no consumer, duplicated by UserApiKey / RouterToken",
    ),
    (
        "User",
        "deleted in S1-D: no consumer, duplicated by UserAccount",
    ),
];

#[test]
fn identity_dtos_are_not_reintroduced_into_common() {
    let text = read("src/types.rs");
    let declared = declared_items(&text);
    let mut violations = Vec::new();
    for (name, owner) in MOVED_OUT {
        if declared
            .iter()
            .any(|d| d == &format!("pub struct {name}") || d == &format!("pub enum {name}"))
        {
            violations.push(format!("{name} (belongs to {owner})"));
        }
    }
    assert!(
        violations.is_empty(),
        "burncloud_common must not redeclare domain types; found: {violations:#?}\n\
         Declared items are:\n{declared:#?}"
    );
}

#[test]
fn common_declares_only_the_known_interfaces_surface() {
    // The expected inventory, with the owner of each item. Anything new has to be classified
    // (usually by moving it to the owning domain rather than keeping it here).
    //
    // ChannelType / Channel / Ability are deliberately absent: since S1-C they are re-exports from
    // the Supply contract, so they are not declarations in this crate.
    const EXPECTED: &[(&str, &str)] = &[
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
    ];

    let mut declared = declared_items(&read("src/types.rs"));
    // The two contract re-export blocks surface as `pub use`, not as declarations.
    let text = read("src/types.rs");
    if text.contains("pub use burncloud_commerce_contracts::pricing::{") {
        declared.push("pub use::commerce".to_string());
    }
    if text.contains("pub use burncloud_supply_contracts::{") {
        declared.push("pub use::supply".to_string());
    }
    declared.sort();
    declared.dedup();

    let expected: Vec<String> = EXPECTED
        .iter()
        .map(|(n, _)| n.to_string())
        .chain([
            "pub use::commerce".to_string(),
            "pub use::supply".to_string(),
        ])
        .collect();
    let mut expected_sorted = expected.clone();
    expected_sorted.sort();
    expected_sorted.dedup();

    assert_eq!(
        declared, expected_sorted,
        "the public inventory of burncloud_common::types changed.\n\
         Add the new item to EXPECTED with its owner, or move it to the owning domain.\n\
         Owners are recorded in #610 and docs/architecture-s1-task-contracts.md"
    );

    // Guard the guard: if this list ever becomes a rubber stamp, it should be obvious.
    assert!(
        EXPECTED.len() >= 8,
        "EXPECTED shrank unexpectedly; the inventory was trimmed without a review"
    );
}

#[test]
fn common_re_exports_the_two_domain_contracts() {
    // The compatibility layer is the only reason domain types are reachable through common.
    let text = read("src/types.rs");
    assert!(
        text.contains("pub use burncloud_commerce_contracts::pricing::{"),
        "the Commerce pricing re-export disappeared"
    );
    assert!(
        text.contains("pub use burncloud_supply_contracts::{"),
        "the Supply re-export disappeared"
    );
    // And none of the crate's files may declare the moved types. Doc comments are excluded, so a
    // documentation example such as `/// pub struct UserRepository { .. }` is not a violation.
    for rel in SRC_FILES {
        let declared = declared_items(&read(rel));
        for (name, owner) in MOVED_OUT {
            for kind in ["pub struct", "pub enum", "pub type"] {
                let decl = format!("{kind} {name}");
                assert!(
                    !declared.iter().any(|d| d == &decl),
                    "{rel} declares {decl}, which belongs to {owner}"
                );
            }
        }
    }
}
