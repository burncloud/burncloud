#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! What this crate exposes to another crate, and the one piece of it that can be tested without aria2
//! (#633, plan item 28).
//!
//! The crate is ~276 lines and had **no tests**. Its plan asks for a download state machine, cancel races,
//! resume policy, progress boundaries, task identity, path normalization and restart reconciliation -- and the
//! crate types it as `U` with a **small local HTTP file** for the `I` half.
//!
//! ## Why this file is about reachability rather than behaviour
//!
//! `DownloadManager`'s methods live in the private `manager` module that `lib.rs` declares:
//!
//! ```text
//! mod error;  mod manager;  mod utils;
//! pub use manager::DownloadManager;
//! ```
//!
//! An inherent `impl` block is not a path item, so whether those methods are callable from another crate is a
//! question about the compiler rather than about the module keyword -- and **this file answers it by calling
//! one**, which is the only honest way. If `DownloadManager::new` is reachable here, the private modules do not
//! block the public API; if it is not, the crate is unusable as a library.
//!
//! ## What cannot be reached from here, and why
//!
//! `extract_filename_from_url` is `pub(crate)` in a private module, so an integration test is the wrong place for
//! it and it is tested inline in `utils.rs` instead.
//!
//! Everything on `DownloadManager` that touches aria2 cannot be exercised here at all: `new()` starts an aria2
//! daemon through `quick_start()`, `add_download` spawns a progress monitor, and the rest make JSON-RPC calls.
//! The plan allows a small local HTTP file for the `I` half, but this crate has no seam for injecting an aria2
//! client -- `DownloadManager` holds a concrete `Arc<Aria2Manager>` and a concrete `Arc<DownloadDB>` -- so the
//! honest position is that the request/response behaviour belongs to `download-aria2` (item 29), whose plan
//! says so, and not to this file.

use burncloud_download::DownloadManager;

/// The type is reachable, which is the minimum a library must offer.
#[test]
fn the_manager_type_is_exported() {
    // A function that names the type, so its reachability is checked at compile time rather than by an import
    // that could be unused.
    fn _accepts(_: &DownloadManager) {}
    println!("`burncloud_download::DownloadManager` is nameable from another crate");
}

/// **The reachability question, answered by the compiler.** `DownloadManager::new` is declared in the private
/// `manager` module. If this test compiles, inherent methods in a private module are reachable through the
/// re-exported type -- which is what the crate needs, since `crates/supply/service-models/examples/
/// test_download.rs` calls it from outside.
///
/// The body is never run: `new()` starts an aria2 daemon and opens the shared default database, neither of which
/// a test may do. Referencing the function without calling it is enough to ask the compiler the question.
#[test]
fn the_manager_methods_are_reachable_through_the_exported_type() {
    // Each is named without being called, which is all the compiler needs to answer the question. The return
    // types are inferred rather than written out, because they mention `DownloadStatus` and the exact error
    // type, and naming those would test this file's transcription rather than the crate's surface.
    //
    // **The first version of this test wrote `fn() -> _` and failed with "mismatched types"** -- these are
    // `async fn`s, so they are functions returning futures. That the failure was a *type* error and not a
    // *privacy* error is the answer this test exists for: an inherent method in a private module **is** reachable
    // through the re-exported type.
    let _ = DownloadManager::new;
    let _ = DownloadManager::add_download;
    let _ = DownloadManager::get_status;
    let _ = DownloadManager::pause;
    let _ = DownloadManager::resume;
    let _ = DownloadManager::remove;
    let _ = DownloadManager::start_progress_monitor;
    let _ = DownloadManager::restore_incomplete_downloads;

    println!("every public method of DownloadManager is nameable from another crate");
}

/// The error type is part of the contract, because every method returns it.
#[test]
fn the_error_type_is_exported_and_carries_both_sources() {
    // `DownloadError` has two `#[from]` variants, one per dependency that can fail. A caller matching on them
    // needs the type to be exported, and it is.
    let database = burncloud_download::DownloadError::Database(
        burncloud_database::DatabaseError::NotInitialized,
    );
    let rendered = database.to_string();
    println!("database error renders as: {rendered}");
    assert!(
        !rendered.is_empty(),
        "the error renders a message rather than an empty string"
    );

    // The two variants are distinguishable, which is what lets a caller retry a database failure differently
    // from an aria2 failure.
    let aria2 = burncloud_download::DownloadError::Aria2(
        burncloud_download_aria2::Aria2Error::RpcError("no client".to_string()),
    );
    assert_ne!(
        std::mem::discriminant(&database),
        std::mem::discriminant(&aria2),
        "the two variants are distinct"
    );
    println!("aria2 error renders as: {aria2}");

    // And `Result<T>` is the crate's alias, so a caller can name the return type.
    fn _accepts(_: burncloud_download::Result<()>) {}
}
