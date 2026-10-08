#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test, and unwrap reports a failed precondition."
)]
//! `PriceCache` region precedence, universal fallback and prefix matching (#633, plan section 5 item 17).
//!
//! The plan's first item for this crate is exactly this, and it is the least covered part: `cache.rs` has
//! two inline tests (`empty` returns `None`, and lookup is case-insensitive), and the other 81 in this
//! crate are in `calculator.rs` and the usage parsers. The plan also warns not to add more average
//! success-path samples, so what follows is the precedence order, which is where a wrong answer changes
//! what a request costs.
//!
//! ## The precedence order, read from `cache.rs:72-104`
//!
//! `get(model, region)` is documented as two independent orders that are in fact nested:
//!
//! ```
//! for candidate in [exact_model, prefix_1, prefix_2, ...]:   // model name, longest first
//!     if region  != ""  and (candidate, region) exists: return it
//!     if                   (candidate, "")     exists: return it
//! ```
//!
//! So the region check and the universal check are **per prefix level**, not global. That is the
//! consequence worth pinning, because it means a universal entry at a *longer* prefix beats a
//! region-specific entry at a *shorter* one.
//!
//! ## The matching rule, which is one-directional
//!
//! The exact requested name is tried first. Only if it has no entry at all does the fallback strip
//! `-`-delimited segments off the **requested** name, from the right, one at a time:
//!
//! ```
//! gpt-4o-2024-11-20  ->  gpt-4o-2024-11  ->  gpt-4o  ->  (stop: no more hyphens)
//! ```
//!
//! So a longer requested name falls back to a shorter cached entry (`gpt-4o-mini` is priced by `gpt-4o`) and
//! **never the reverse**: `gpt-4o-2024-11-20` being priced does not make `gpt-4o` priced. An earlier version
//! of this file asserted the reverse and failed, which is how the direction was settled -- by measurement,
//! not by reading the doc comment, which is ambiguous on the point.
//!
//! Hyphens are the only boundary; a dot is not, even though model names use both.
//!
//! ## Two things the tests do not assume
//!
//! The cache is keyed on `(lowercase model, region)` with no currency, which is consistent with the
//! database: `BillingPriceModel::upsert` declares `ON CONFLICT(model, region)`, so at most one row exists
//! per `(model, region)` and `currency` is a column of that row rather than part of its identity. A test
//! that seeded two currencies for one region would therefore be testing a state the schema forbids.
//!
//! `refresh` builds its map with `entry(..).or_insert(..)`, so if the loader ever returned two rows with
//! the same `(model, region)` key the first would win silently. That is unreachable through `upsert` for
//! the reason above; the tests below pin the reachable half -- that a refresh **replaces** rather than
//! merges, including for a price that was deleted.

use burncloud_commerce_contracts::pricing::Price;
use burncloud_database::{create_database_with_url, Database};
use burncloud_database_billing::{BillingPriceModel, PriceInput};
use burncloud_service_billing::cache::PriceCache;

/// A fresh SQLite file database with the real migrations applied.
///
/// A file rather than `:memory:`: the pool opens several connections and an in-memory SQLite database is
/// per-connection, so migrations and the cache would see different databases. Three slashes because the
/// path is absolute; the two-slash form is not a URL SQLite can open on Windows.
async fn fresh_db(tag: &str) -> (Database, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "bc_pricecache_{}_{}_{}.db",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::remove_file(&path).ok();
    let normalized = path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{}?mode=rwc", normalized);
    let db = create_database_with_url(&url)
        .await
        .unwrap_or_else(|e| panic!("open {url}: {e}"));
    (db, path)
}

/// Close the pool, wait for the handle to be released, then remove the database and its WAL companions.
async fn cleanup(db: Database, path: std::path::PathBuf) {
    db.close().await.ok();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    for suffix in ["", "-wal", "-shm"] {
        let mut candidate = path.clone().into_os_string();
        candidate.push(suffix);
        let candidate = std::path::PathBuf::from(candidate);
        if candidate.exists() {
            std::fs::remove_file(&candidate).unwrap_or_else(|e| {
                panic!(
                    "test database file {} was not removed ({e}); the temp directory would fill up",
                    candidate.display()
                )
            });
        }
    }
}

/// Seed one price. Every unset field is `None` deliberately: these tests are about which price is
/// selected, not about which optional price columns exist.
fn price_input(model: &str, region: Option<&str>, input: i64) -> PriceInput {
    PriceInput {
        model: model.to_string(),
        input_price: input,
        output_price: input,
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
        region: region.map(|r| r.to_string()),
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

/// The `input_price` a lookup selected, or `None`. Only the price matters here, so the assertion reads one
/// field rather than comparing whole `Price` values whose `id` and timestamps are incidental.
async fn selected(cache: &PriceCache, model: &str, region: Option<&str>) -> Option<i64> {
    cache.get(model, region).await.map(|p: Price| p.input_price)
}

// -------------------------------------------------------------------------------------------
// region precedence and universal fallback
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_region_specific_price_beats_the_universal_one() {
    // The core rule. If it were reversed, every region would silently pay the universal price.
    let (db, path) = fresh_db("region_first").await;

    BillingPriceModel::upsert(&db, &price_input("regmodel", None, 100))
        .await
        .expect("universal");
    BillingPriceModel::upsert(&db, &price_input("regmodel", Some("us"), 200))
        .await
        .expect("regional");

    let cache = PriceCache::load(&db).await.expect("load");

    assert_eq!(
        selected(&cache, "regmodel", Some("us")).await,
        Some(200),
        "a region with its own price must use it, not the universal fallback"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn a_region_without_its_own_price_falls_back_to_the_universal_one() {
    // The other half: an unlisted region must still be priced. Without the fallback the request would be
    // unpriced, which is the "missing price" case the plan wants to fail closed rather than guess at.
    let (db, path) = fresh_db("universal_fallback").await;

    BillingPriceModel::upsert(&db, &price_input("fallbackmodel", None, 100))
        .await
        .expect("universal");
    BillingPriceModel::upsert(&db, &price_input("fallbackmodel", Some("us"), 200))
        .await
        .expect("regional");

    let cache = PriceCache::load(&db).await.expect("load");

    assert_eq!(
        selected(&cache, "fallbackmodel", Some("eu")).await,
        Some(100),
        "a region with no entry of its own must fall back to the universal price"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn asking_for_no_region_uses_the_universal_price_only() {
    // `None` and `""` both mean "no region". A region-specific price must not be reachable through them, or
    // a caller that forgot to pass a region would be charged an arbitrary region's price.
    let (db, path) = fresh_db("no_region").await;

    BillingPriceModel::upsert(&db, &price_input("nomodel", Some("us"), 200))
        .await
        .expect("regional");
    BillingPriceModel::upsert(&db, &price_input("nomodel", None, 100))
        .await
        .expect("universal");

    let cache = PriceCache::load(&db).await.expect("load");

    assert_eq!(selected(&cache, "nomodel", None).await, Some(100));
    assert_eq!(selected(&cache, "nomodel", Some("")).await, Some(100));

    cleanup(db, path).await;
}

#[tokio::test]
async fn a_model_with_only_a_regional_price_is_unpriced_without_that_region() {
    // Recorded because it is a real routing consequence: the price exists, but a caller that does not know
    // the region cannot find it. Whether that should fall back to anything is a policy question -- the cache
    // deliberately does not invent a price -- so this pins the behaviour rather than endorsing it.
    let (db, path) = fresh_db("regional_only").await;

    BillingPriceModel::upsert(&db, &price_input("regionalonly", Some("us"), 200))
        .await
        .expect("regional");

    let cache = PriceCache::load(&db).await.expect("load");

    assert_eq!(
        selected(&cache, "regionalonly", Some("us")).await,
        Some(200)
    );
    assert_eq!(
        selected(&cache, "regionalonly", None).await,
        None,
        "with no universal entry and no region given, there is nothing to select"
    );

    cleanup(db, path).await;
}

// -------------------------------------------------------------------------------------------
// exact versus prefix
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn an_exact_model_name_beats_a_prefix_match() {
    let (db, path) = fresh_db("exact_first").await;

    BillingPriceModel::upsert(&db, &price_input("gpt-4o-2024-11-20", None, 300))
        .await
        .expect("exact");
    BillingPriceModel::upsert(&db, &price_input("gpt-4o", None, 100))
        .await
        .expect("prefix");

    let cache = PriceCache::load(&db).await.expect("load");

    assert_eq!(
        selected(&cache, "gpt-4o-2024-11-20", None).await,
        Some(300),
        "the exact entry must win over the prefix it would otherwise match"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn a_longer_prefix_beats_a_shorter_one() {
    // The loop strips one `-segment` at a time, so `gpt-4o-2024-11` is tried before `gpt-4o`. A dated
    // variant of a model should therefore price against the nearest ancestor that has a price.
    let (db, path) = fresh_db("longest_prefix").await;

    BillingPriceModel::upsert(&db, &price_input("gpt-4o-2024-11", None, 250))
        .await
        .expect("longer prefix");
    BillingPriceModel::upsert(&db, &price_input("gpt-4o", None, 100))
        .await
        .expect("shorter prefix");

    let cache = PriceCache::load(&db).await.expect("load");

    assert_eq!(
        selected(&cache, "gpt-4o-2024-11-20", None).await,
        Some(250),
        "the nearest ancestor with a price must be selected"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn a_universal_entry_at_a_longer_prefix_beats_a_regional_entry_at_a_shorter_one() {
    // The consequence of the nesting described in the module docs, and the one outcome of this order that is
    // not obvious: the region check happens **per prefix level**, so `(gpt-4o-2024-11, "")` is selected
    // ahead of `(gpt-4o, "us")`.
    //
    // This is asserted as the behaviour, not as what is desirable. A reader who expects "regional always
    // beats universal" would predict 999 here and be wrong, and since the outcome changes what a request
    // costs, the expectation is worth recording loudly. Changing it is a decision for whoever owns pricing
    // policy, not something to fix silently in a test.
    let (db, path) = fresh_db("nested_precedence").await;

    BillingPriceModel::upsert(&db, &price_input("gpt-4o", Some("us"), 999))
        .await
        .expect("regional, shorter prefix");
    BillingPriceModel::upsert(&db, &price_input("gpt-4o-2024-11", None, 250))
        .await
        .expect("universal, longer prefix");

    let cache = PriceCache::load(&db).await.expect("load");

    assert_eq!(
        selected(&cache, "gpt-4o-2024-11-20", Some("us")).await,
        Some(250),
        "the prefix loop tries the longer prefix first, and its universal entry is found before the \
         shorter prefix's regional entry is ever considered"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn a_lookup_that_shares_no_prefix_with_an_entry_is_none() {
    // Prefix stripping is the only fallback, so a name that is not an extension of any entry must not match.
    // An accidental match is worse than a miss: it is a wrong price rather than a missing one, and nothing
    // downstream can tell.
    //
    // **The direction matters and an earlier version of this test got it backwards.** The rule is
    // one-directional: a *longer* requested name falls back to a *shorter* cached entry
    // (`gpt-4o-mini` -> `gpt-4o`), never the other way. So `gpt-4o-mini` matching `gpt-4o` is correct, and
    // the assertions below are about names that genuinely share nothing.
    let (db, path) = fresh_db("no_match").await;

    BillingPriceModel::upsert(&db, &price_input("gpt-4o", None, 100))
        .await
        .expect("gpt-4o");
    BillingPriceModel::upsert(&db, &price_input("claude-3", None, 500))
        .await
        .expect("claude-3");

    let cache = PriceCache::load(&db).await.expect("load");

    assert_eq!(selected(&cache, "llama-3-70b", None).await, None);
    assert_eq!(
        selected(&cache, "gpt", None).await,
        None,
        "an extension of an entry is not an entry"
    );
    assert_eq!(
        selected(&cache, "gpt-3.5", None).await,
        None,
        "a sibling name must not match gpt-4o merely by sharing characters"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn a_longer_requested_name_falls_back_to_its_cached_prefix() {
    // The intended direction, stated as its own test because the previous one got it wrong and the
    // distinction is easy to invert: the fallback strips segments off the **requested** name, so
    // `gpt-4o-mini` is priced by `gpt-4o`.
    //
    // This is deliberate and useful -- a provider that adds a variant does not need a price row before it can
    // be billed -- and it is also the rule with the most billing impact, because it decides that an unknown
    // model is charged at its family's rate rather than failing closed.
    let (db, path) = fresh_db("extension_fallback").await;

    BillingPriceModel::upsert(&db, &price_input("gpt-4o", None, 100))
        .await
        .expect("gpt-4o");

    let cache = PriceCache::load(&db).await.expect("load");

    assert_eq!(
        selected(&cache, "gpt-4o-mini", None).await,
        Some(100),
        "a longer name must fall back to its shorter cached prefix"
    );
    assert_eq!(
        selected(&cache, "gpt-4o-mini-2024-07-18", None).await,
        Some(100),
        "and it keeps stripping until something matches"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn a_model_with_no_hyphen_falls_back_only_along_its_hyphen_boundaries() {
    // The loop is driven by `rfind('-')`, so segment boundaries are hyphens and nothing else. A name with no
    // hyphen has no fallback at all, and a dot does not create one -- worth stating because model names use
    // both separators, and only one of them participates.
    let (db, path) = fresh_db("no_hyphen").await;

    BillingPriceModel::upsert(&db, &price_input("plainmodel", None, 100))
        .await
        .expect("plain");

    let cache = PriceCache::load(&db).await.expect("load");

    assert_eq!(selected(&cache, "plainmodel", None).await, Some(100));
    assert_eq!(
        selected(&cache, "plainmodel-v2", None).await,
        Some(100),
        "a hyphen boundary is a fallback point, so the variant matches its base"
    );
    assert_eq!(
        selected(&cache, "plainmodel.v2", None).await,
        None,
        "a dot is not a boundary, so this name shares no prefix with `plainmodel` under the rule"
    );

    cleanup(db, path).await;
}

// -------------------------------------------------------------------------------------------
// refresh
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn refresh_replaces_the_cache_rather_than_merging_into_it() {
    // `refresh` builds a new map and assigns it under the write lock, so a price removed from the database
    // disappears from the cache. If it merged instead, a deleted or repriced row would keep being billed.
    let (db, path) = fresh_db("refresh_replaces").await;

    BillingPriceModel::upsert(&db, &price_input("bumpmodel", None, 100))
        .await
        .expect("first");
    let cache = PriceCache::load(&db).await.expect("load");
    assert_eq!(selected(&cache, "bumpmodel", None).await, Some(100));

    // The identity is `(model, region)`, so this updates the same row rather than adding one.
    BillingPriceModel::upsert(&db, &price_input("bumpmodel", None, 700))
        .await
        .expect("reprice");

    // The cache is stale until refreshed: this is what makes the refresh necessary rather than optional.
    assert_eq!(
        selected(&cache, "bumpmodel", None).await,
        Some(100),
        "a cached price must not change on its own when the database changes"
    );

    cache.refresh(&db).await.expect("refresh");

    assert_eq!(
        selected(&cache, "bumpmodel", None).await,
        Some(700),
        "after a refresh the new price must be the one selected"
    );
    assert_eq!(
        cache.get("bumpmodel", None).await.map(|p| p.id),
        cache.get("bumpmodel", None).await.map(|p| p.id),
        "the refreshed entry is stable across reads"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn refresh_drops_a_price_that_no_longer_exists_in_the_database() {
    // A **removal**, which is the half the repricing test above cannot see: a refresh that merged into the old
    // map instead of replacing it would still pick up a changed price, so only a deletion distinguishes
    // "replace" from "merge". Measured -- a mutation replacing the assignment with `guard.extend(map)` passed
    // every other test in this file.
    //
    // It matters for billing because a removed price row is how a model is retired or corrected. A cache that
    // keeps serving it charges at a rate the database no longer holds.
    let (db, path) = fresh_db("refresh_removes").await;

    BillingPriceModel::upsert(&db, &price_input("keepmodel", None, 100))
        .await
        .expect("keep");
    BillingPriceModel::upsert(&db, &price_input("dropmodel", None, 900))
        .await
        .expect("drop");

    let cache = PriceCache::load(&db).await.expect("load");
    assert_eq!(selected(&cache, "dropmodel", None).await, Some(900));
    assert_eq!(selected(&cache, "keepmodel", None).await, Some(100));

    BillingPriceModel::delete_all_for_model(&db, "dropmodel")
        .await
        .expect("delete");
    cache.refresh(&db).await.expect("refresh");

    assert_eq!(
        selected(&cache, "dropmodel", None).await,
        None,
        "a price deleted from the database must be gone after a refresh; if it is still served, the cache \
         merged rather than replaced and is billing a rate that no longer exists"
    );
    assert_eq!(
        selected(&cache, "keepmodel", None).await,
        Some(100),
        "and the unrelated entry must survive the refresh"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn a_refresh_that_cannot_read_the_database_leaves_the_old_prices_intact() {
    // "Atomic refresh" in the doc comment means the old map survives a failed load rather than being emptied.
    // That matters: an empty cache prices nothing, and a caller cannot tell "no price configured" from "the
    // price table went away", so a transient database failure would silently become unpriced requests.
    //
    // The failure is produced by dropping the price table, which makes the read fail for a real reason
    // while leaving the `Database` handle usable. Closing the handle instead would consume it
    // (`Database::close` takes `self`), so the refresh could not be attempted afterwards.
    let (db, path) = fresh_db("refresh_failure").await;

    BillingPriceModel::upsert(&db, &price_input("survivemodel", None, 4242))
        .await
        .expect("seed");
    let cache = PriceCache::load(&db).await.expect("load");
    assert_eq!(selected(&cache, "survivemodel", None).await, Some(4242));

    burncloud_database::sqlx::query("DROP TABLE billing_prices")
        .execute(db.get_connection().expect("connection").pool())
        .await
        .expect("dropping the price table");

    // The read must fail now, or this test is not exercising the failure path at all.
    assert!(
        BillingPriceModel::list(&db, 10, 0, None, None).await.is_err(),
        "the price table is gone, so listing prices must fail; if this succeeds the failure path below is \
         never reached and the test would pass for the wrong reason"
    );

    let refresh_result = cache.refresh(&db).await;
    assert!(
        refresh_result.is_err(),
        "refreshing after the price table was dropped must report the failure rather than silently \
         succeeding"
    );

    assert_eq!(
        selected(&cache, "survivemodel", None).await,
        Some(4242),
        "a failed refresh must leave the prices it could not replace in place; an emptied cache would \
         price nothing and the caller cannot tell the difference from 'no price configured'"
    );

    // The price table is gone but the `Database` handle is alive, so the usual cleanup works. This call was
    // lost in an edit and the suite leaked one database per run; measured at one leftover file per run.
    cleanup(db, path).await;
}

#[tokio::test]
async fn loading_an_empty_database_yields_an_empty_cache_that_still_answers() {
    // The boundary: no prices at all. `load` must succeed with an empty cache rather than error, and a
    // lookup must be `None` rather than a panic, because that is the state of a fresh installation.
    let (db, path) = fresh_db("empty_db").await;

    let cache = PriceCache::load(&db)
        .await
        .expect("load on an empty database");
    assert_eq!(selected(&cache, "anything", None).await, None);
    assert_eq!(selected(&cache, "anything", Some("us")).await, None);

    cleanup(db, path).await;
}
