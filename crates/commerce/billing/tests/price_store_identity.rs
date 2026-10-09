#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic_in_result_fn,
    reason = "Test-only file: the assertions are the test, and clippy.toml's allow-panic-in-tests does not recognise #[tokio::test]"
)]
//! The billing price store's identity, precision and update scoping (#633, plan section 5 item 16).
//!
//! `commerce/billing` is the persistence layer under the price cache, so a mistake here moves real money.
//! `tests/sqlite_row_mapping.rs` and the inline tests in `rows.rs` cover reading a row; this file covers what
//! the plan lists as missing: upsert behaviour, the model/currency/region identity, the region column, 64-bit
//! round-tripping, tier persistence, filtering and pagination, update scoping, and the two SQL dialects.
//!
//! ## The identity is not what the plan describes, and that is the finding
//!
//! The plan asks for "model/currency/region 组合键" and "NULL region". Measured against
//! `0010_rename_tables.sql:151`:
//!
//! ```sql
//! currency TEXT NOT NULL DEFAULT 'USD',
//! region   TEXT NOT NULL DEFAULT '',        -- not nullable
//! UNIQUE(model, region)                     -- currency is not part of the key
//! ```
//!
//! and `billing_price.rs` writes with `ON CONFLICT(model, region) DO UPDATE` but reads with
//! `WHERE model = ? AND currency = ? AND region = ?`. Three consequences, each measured below:
//!
//! 1. **One row per model and region, whatever the currency.** A second currency for the same model and region
//!    **overwrites** the first rather than adding a row.
//! 2. **Reading a currency that was overwritten returns no price**, even though a row exists for that model and
//!    region -- the write conflict target and the read predicate disagree about what the identity is.
//! 3. **The region is a string, never NULL**, with `""` standing for "no region". `None` and `Some("")` are
//!    therefore the same row, and neither is a SQL NULL.
//!
//! None of this is asserted as correct. It is recorded, because a caller reading the plan would expect to be
//! able to store a USD and a CNY price for one model and would silently lose one of them.
//!
//! ## Monthly quota subscription status
//!
//! The historical migration `0015_monthly_quota.sql` creates `billing_plans` and
//! `billing_subscriptions` for both database backends. That schema is **not an active runtime
//! capability** today: the orphan `billing_plan.rs` / `billing_subscription.rs` implementations
//! were never part of a module, referenced types that did not exist, and were removed in #653.
//!
//! No Rust plan/subscription types, database modules or service/API are currently exported.
//! Completing that feature is tracked by #652 and must start by defining the contract and quota
//! semantics rather than re-enabling the deleted files.
//!

use burncloud_commerce_contracts::pricing::{PriceInput, TieredPriceInput};
use burncloud_database::{create_database_with_url, Database};
use burncloud_commerce_billing::{BillingPriceModel, BillingTieredPriceModel};
use std::error::Error;

/// A fresh database with the real migrations applied.
///
/// Three slashes and forward slashes: `sqlite:///C:/...` is absolute on Windows, whereas the two-slash form is
/// not a URL SQLite can open and fails with "(code: 14) unable to open database file" before any assertion.
async fn fresh_db(tag: &str) -> Result<(Database, std::path::PathBuf), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!(
        "bc_bp_{}_{}_{}.db",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::remove_file(&path).ok();
    let normalized = path.to_string_lossy().replace('\\', "/");
    let db = create_database_with_url(&format!("sqlite:///{}?mode=rwc", normalized)).await?;
    Ok((db, path))
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
            std::fs::remove_file(&candidate)
                .unwrap_or_else(|e| panic!("{} was not removed ({e})", candidate.display()));
        }
    }
}

/// A price row naming only what a test asserts on.
fn price(model: &str, currency: &str, region: Option<&str>, input: i64, output: i64) -> PriceInput {
    PriceInput {
        model: model.to_string(),
        currency: currency.to_string(),
        region: region.map(str::to_string),
        input_price: input,
        output_price: output,
        ..Default::default()
    }
}

/// Every row the store holds for one model, read without naming a currency.
async fn rows_for(
    db: &Database,
    model: &str,
) -> Result<Vec<(String, String, i64)>, Box<dyn Error>> {
    let conn = db.get_connection()?;
    let rows: Vec<(String, String, i64)> = burncloud_database::sqlx::query_as(
        "SELECT currency, region, input_price FROM billing_prices WHERE model = ? ORDER BY region",
    )
    .bind(model)
    .fetch_all(conn.pool())
    .await?;
    Ok(rows)
}

// -------------------------------------------------------------------------------------------
// upsert and the identity, which is (model, region)
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn an_upsert_is_visible_through_both_readers() -> Result<(), Box<dyn Error>> {
    // The baseline: a row written by `upsert` reads back, so the later tests compare two real values rather
    // than two failures.
    let (db, path) = fresh_db("upsert_read").await?;
    BillingPriceModel::upsert(&db, &price("gpt-4o", "USD", None, 2_500_000, 10_000_000)).await?;

    let found = BillingPriceModel::get(&db, "gpt-4o", "USD", None)
        .await?
        .unwrap_or_else(|| panic!("the row must be readable by model/currency/region"));
    println!(
        "read back: input={} output={}",
        found.input_price, found.output_price
    );
    assert_eq!(found.input_price, 2_500_000);
    assert_eq!(found.output_price, 10_000_000);

    assert!(
        BillingPriceModel::get_by_model_region(&db, "gpt-4o", None)
            .await?
            .is_some(),
        "get_by_model_region finds the same row"
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn a_second_currency_for_the_same_model_and_region_overwrites_the_first(
) -> Result<(), Box<dyn Error>> {
    // **Consequence 1 of `UNIQUE(model, region)`.** The plan describes the key as model/currency/region, so the
    // natural reading is that a model can hold a USD price and a CNY price side by side. It cannot: the second
    // upsert updates the same row, and the first currency's price is gone.
    let (db, path) = fresh_db("currency_overwrite").await?;

    BillingPriceModel::upsert(&db, &price("multi", "USD", None, 1_000_000, 2_000_000)).await?;
    BillingPriceModel::upsert(&db, &price("multi", "CNY", None, 7_000_000, 8_000_000)).await?;

    let rows = rows_for(&db, "multi").await?;
    println!("rows after writing USD then CNY at the same region: {rows:?}");

    assert_eq!(
        rows.len(),
        1,
        "one row, not two: uniqueness is on (model, region), so the second currency updated the first"
    );
    assert_eq!(
        rows[0].0, "CNY",
        "and the surviving row carries the later currency"
    );
    assert_eq!(rows[0].2, 7_000_000, "with the later price");

    // The user-visible half: asking for the USD price of a model that has one returns nothing.
    let usd = BillingPriceModel::get(&db, "multi", "USD", None).await?;
    println!("USD price after the CNY upsert: {usd:?}");
    assert!(
        usd.is_none(),
        "the read predicate includes `currency`, which the write conflict target does not, so a USD lookup \
         finds no row even though a row exists for this model and region"
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn a_different_region_is_a_different_row_and_keeps_its_own_currency(
) -> Result<(), Box<dyn Error>> {
    // The half of the composite key that **does** work. `region` is in the unique constraint, so the same model
    // in two regions holds two rows with independent currencies and prices -- which is the basis of the regional
    // price precedence the service layer builds on.
    let (db, path) = fresh_db("region_key").await?;
    BillingPriceModel::upsert(
        &db,
        &price("regional", "USD", Some("us-east-1"), 1_000_000, 0),
    )
    .await?;
    BillingPriceModel::upsert(
        &db,
        &price("regional", "CNY", Some("cn-north-1"), 7_000_000, 0),
    )
    .await?;

    let rows = rows_for(&db, "regional").await?;
    println!("rows: {rows:?}");
    assert_eq!(rows.len(), 2, "two regions, two rows");

    assert_eq!(
        BillingPriceModel::get(&db, "regional", "USD", Some("us-east-1"))
            .await?
            .unwrap_or_else(|| panic!("the us-east-1 row must exist"))
            .input_price,
        1_000_000
    );
    assert_eq!(
        BillingPriceModel::get(&db, "regional", "CNY", Some("cn-north-1"))
            .await?
            .unwrap_or_else(|| panic!("the cn-north-1 row must exist"))
            .input_price,
        7_000_000
    );

    // And **the currency fallback**: asking for a currency a region does not have returns that region's USD
    // price rather than nothing. `get` tries the exact `(model, currency, region)` and, for any currency other
    // than USD, then tries `(model, 'USD', region)` (`billing_price.rs:67-107`).
    let cny_at_us_east = BillingPriceModel::get(&db, "regional", "CNY", Some("us-east-1")).await?;
    println!("CNY at us-east-1 (which holds USD): {cny_at_us_east:?}");
    assert_eq!(
        cny_at_us_east.map(|p| (p.currency, p.input_price)),
        Some(("USD".to_string(), 1_000_000)),
        "a CNY lookup in a region that only holds USD returns the USD row, and the row reports its own \
         currency -- so a caller can see which price it actually got"
    );

    // The fallback is one-directional: a **USD** lookup never borrows another currency's price, because the
    // branch only runs when the requested currency is not USD.
    assert!(
        BillingPriceModel::get(&db, "regional", "USD", Some("cn-north-1")).await?.is_none(),
        "a USD lookup in a region that holds only CNY returns nothing rather than borrowing the CNY price"
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn the_region_is_a_string_so_a_missing_region_is_the_empty_one() -> Result<(), Box<dyn Error>>
{
    // **Consequence 3: the column is `TEXT NOT NULL DEFAULT ''`, not nullable.** The plan asks about a "NULL
    // region"; there is no NULL. `None` and `Some("")` both become `''`, so they are the same row and two
    // upserts of them collapse rather than conflicting.
    let (db, path) = fresh_db("region_string").await?;

    BillingPriceModel::upsert(&db, &price("noregion", "USD", None, 100, 200)).await?;
    BillingPriceModel::upsert(&db, &price("noregion", "USD", Some(""), 300, 400)).await?;

    let rows = rows_for(&db, "noregion").await?;
    println!("rows for None then Some(\"\"): {rows:?}");
    assert_eq!(rows.len(), 1, "None and Some(\"\") are the same row");
    assert_eq!(
        rows[0].1, "",
        "and the stored region is the empty string, not NULL"
    );
    assert_eq!(rows[0].2, 300, "so the second upsert updated the first");

    // Reading with `None` finds the row written with `Some("")` and vice versa, which is the point of the
    // normalisation.
    assert!(BillingPriceModel::get(&db, "noregion", "USD", None)
        .await?
        .is_some());
    assert!(BillingPriceModel::get(&db, "noregion", "USD", Some(""))
        .await?
        .is_some());

    // The column really is NOT NULL: a direct insert of NULL must be rejected by the schema rather than stored.
    let conn = db.get_connection()?;
    let null_insert = burncloud_database::sqlx::query(
        "INSERT INTO billing_prices (model, currency, region) VALUES ('nullregion', 'USD', NULL)",
    )
    .execute(conn.pool())
    .await;
    match &null_insert {
        Ok(_) => panic!("the schema accepted a NULL region, so the column is nullable after all"),
        Err(e) => println!("a NULL region is rejected by the schema: {e}"),
    }
    assert!(null_insert.is_err(), "region must be NOT NULL");

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn the_full_64_bit_range_survives_a_round_trip() -> Result<(), Box<dyn Error>> {
    // The plan's "高精度整数". Prices are nanodollars in `i64`, and a value that passed through a narrower type
    // or a float would come back changed. Both extremes are used, because a truncation shows at one end and a
    // sign error at the other.
    let (db, path) = fresh_db("i64").await?;
    for (label, value) in [
        ("i64::MAX", i64::MAX),
        ("i64::MIN", i64::MIN),
        ("i64::MAX - 1", i64::MAX - 1),
        ("-1", -1),
        ("a 19-digit figure", 9_223_372_036_854_775_806_i64),
    ] {
        BillingPriceModel::upsert(&db, &price("wide", "USD", Some(label), value, value)).await?;
        let got = BillingPriceModel::get(&db, "wide", "USD", Some(label))
            .await?
            .unwrap_or_else(|| panic!("{label} must be readable"))
            .input_price;
        println!("{label}: wrote {value}, read {got}");
        assert_eq!(got, value, "{label} must round-trip exactly");
    }

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn the_currency_fallback_runs_one_way_and_stays_inside_its_region(
) -> Result<(), Box<dyn Error>> {
    // **Two gaps a mutation found.** The fallback at `billing_price.rs:67-107` is guarded by
    // `if currency != "USD"`, and the fallback query carries the region. Two mutations survived the first
    // version of this file:
    //
    //  * dropping the guard (`if true`) -- survived, because in every earlier fixture a USD row existed, so a
    //    USD lookup found its own row and never reached the fallback;
    //  * dropping the region from the fallback query -- survived, because no earlier fixture had two regions both
    //    holding USD.
    //
    // This test builds exactly those two shapes.
    let (db, path) = fresh_db("fallback_bounds").await?;

    // A model whose only price is CNY. A USD lookup must find **nothing**: the fallback must not run for USD, and
    // there is no USD row to find.
    BillingPriceModel::upsert(&db, &price("cny-only", "CNY", None, 7_000_000, 0)).await?;
    let usd_lookup = BillingPriceModel::get(&db, "cny-only", "USD", None).await?;
    println!("USD lookup on a CNY-only model: {usd_lookup:?}");
    assert!(
        usd_lookup.is_none(),
        "the fallback is one-directional: a USD lookup must not borrow the CNY price"
    );

    // And the other direction still works, so the assertion above is about the guard rather than about the
    // model having no price at all.
    let cny_lookup = BillingPriceModel::get(&db, "cny-only", "CNY", None).await?;
    assert_eq!(
        cny_lookup.map(|p| p.currency),
        Some("CNY".to_string()),
        "and a CNY lookup finds its own row"
    );

    // Two regions, each holding a different USD price, and a third region with nothing.
    BillingPriceModel::upsert(
        &db,
        &price("two-usd-regions", "USD", Some("us-east-1"), 1_000_000, 0),
    )
    .await?;
    BillingPriceModel::upsert(
        &db,
        &price("two-usd-regions", "USD", Some("eu-west-1"), 2_000_000, 0),
    )
    .await?;

    // A CNY lookup in each region must return **that region's** USD row, not the other's.
    let east = BillingPriceModel::get(&db, "two-usd-regions", "CNY", Some("us-east-1")).await?;
    let west = BillingPriceModel::get(&db, "two-usd-regions", "CNY", Some("eu-west-1")).await?;
    println!(
        "CNY in us-east-1 -> {:?}, CNY in eu-west-1 -> {:?}",
        east.as_ref().map(|p| (p.currency.clone(), p.input_price)),
        west.as_ref().map(|p| (p.currency.clone(), p.input_price))
    );
    assert_eq!(
        east.map(|p| p.input_price),
        Some(1_000_000),
        "the fallback stays inside the requested region"
    );
    assert_eq!(
        west.map(|p| p.input_price),
        Some(2_000_000),
        "and returns the other region's price for the other region"
    );

    // A region that holds nothing at all returns nothing, even though USD rows exist elsewhere -- which is the
    // same property from the other side.
    let absent = BillingPriceModel::get(&db, "two-usd-regions", "CNY", Some("ap-south-1")).await?;
    println!("CNY in a region with nothing: {absent:?}");
    assert!(
        absent.is_none(),
        "the currency fallback does not reach across regions, so an unpriced region stays unpriced"
    );

    cleanup(db, path).await;
    Ok(())
}

// -------------------------------------------------------------------------------------------
// update scoping
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn updating_the_cache_prices_targets_a_model_and_region_and_reports_whether_it_matched(
) -> Result<(), Box<dyn Error>> {
    // **The signature is part of the finding.** `update_cache_pricing(db, model, region, read, creation)` takes
    // no currency -- it updates the row for that model and region and returns whether it matched anything. I had
    // assumed a currency parameter; the signature says otherwise, which is consistent with `(model, region)`
    // being the key.
    let (db, path) = fresh_db("cache_scope").await?;
    BillingPriceModel::upsert(&db, &price("scoped", "USD", None, 1_000_000, 1_000_000)).await?;
    BillingPriceModel::upsert(
        &db,
        &price("scoped", "USD", Some("eu-west-1"), 2_000_000, 2_000_000),
    )
    .await?;

    let matched =
        BillingPriceModel::update_cache_pricing(&db, "scoped", None, Some(111), Some(222)).await?;
    assert!(matched, "the model/region matched a row");

    let universal = BillingPriceModel::get(&db, "scoped", "USD", None)
        .await?
        .unwrap_or_else(|| panic!("the universal row must exist"));
    let eu = BillingPriceModel::get(&db, "scoped", "USD", Some("eu-west-1"))
        .await?
        .unwrap_or_else(|| panic!("the eu-west-1 row must exist"));
    println!(
        "universal cache ({:?}, {:?}), eu input {}",
        universal.cache_read_input_price, universal.cache_creation_input_price, eu.input_price
    );

    assert_eq!(
        universal.cache_read_input_price,
        Some(111),
        "the targeted row was updated"
    );
    assert_eq!(universal.cache_creation_input_price, Some(222));
    assert_eq!(
        eu.input_price, 2_000_000,
        "the same model in another region is untouched, which is the scope the signature expresses"
    );

    // A region nothing matches reports false rather than failing, which is what the `bool` is for.
    let nothing =
        BillingPriceModel::update_cache_pricing(&db, "scoped", Some("ap-south-1"), Some(1), None)
            .await?;
    println!("an unmatched region reports {nothing}");
    assert!(!nothing, "no row matched, reported rather than raised");

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn updating_batch_prices_does_not_change_the_standard_prices() -> Result<(), Box<dyn Error>> {
    // The batch price is a separate pair of columns, and a store that wrote them into `input_price` /
    // `output_price` would halve the standard rate for every request rather than only for batch ones.
    let (db, path) = fresh_db("batch_scope").await?;
    BillingPriceModel::upsert(&db, &price("batched", "USD", None, 1_000_000, 2_000_000)).await?;

    let matched = BillingPriceModel::update_batch_pricing(
        &db,
        "batched",
        None,
        Some(500_000),
        Some(1_000_000),
    )
    .await?;
    assert!(matched, "the model/region matched a row");

    let got = BillingPriceModel::get(&db, "batched", "USD", None)
        .await?
        .unwrap_or_else(|| panic!("the row must still be readable"));
    println!(
        "standard ({}, {}) batch ({:?}, {:?})",
        got.input_price, got.output_price, got.batch_input_price, got.batch_output_price
    );

    assert_eq!(
        got.input_price, 1_000_000,
        "the standard input price is unchanged"
    );
    assert_eq!(got.output_price, 2_000_000, "and the standard output price");
    assert_eq!(
        got.batch_input_price,
        Some(500_000),
        "the batch price was written"
    );
    assert_eq!(got.batch_output_price, Some(1_000_000));

    cleanup(db, path).await;
    Ok(())
}

// -------------------------------------------------------------------------------------------
// filtering and pagination
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn listing_pages_and_filters_by_currency_and_region() -> Result<(), Box<dyn Error>> {
    // The plan's "筛选分页". `list(db, limit, offset, currency, region)` carries both filters; a limit that was
    // ignored, or an offset that did not move, shows up as the wrong count rather than as an error.
    let (db, path) = fresh_db("paging").await?;
    for i in 0..7 {
        BillingPriceModel::upsert(&db, &price(&format!("m{i}"), "USD", None, i as i64, 0)).await?;
    }
    BillingPriceModel::upsert(&db, &price("cny-model", "CNY", None, 99, 0)).await?;
    BillingPriceModel::upsert(&db, &price("eu-model", "USD", Some("eu-west-1"), 98, 0)).await?;

    let all = BillingPriceModel::list(&db, 100, 0, None, None).await?;
    assert_eq!(all.len(), 9, "every row");

    let first = BillingPriceModel::list(&db, 3, 0, None, None).await?;
    let second = BillingPriceModel::list(&db, 3, 3, None, None).await?;
    let third = BillingPriceModel::list(&db, 3, 6, None, None).await?;
    println!("pages: {} {} {}", first.len(), second.len(), third.len());
    assert_eq!(first.len(), 3, "the limit is honoured");
    assert_eq!(second.len(), 3, "and the offset moves");
    assert_eq!(third.len(), 3, "the last page holds the remainder");

    // The pages must not overlap, or "pagination" is the same rows repeatedly.
    let mut ids: Vec<String> = first
        .iter()
        .chain(second.iter())
        .chain(third.iter())
        .map(|p| {
            format!(
                "{}/{}/{}",
                p.model,
                p.currency,
                p.region.as_deref().unwrap_or("")
            )
        })
        .collect();
    let total = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), total, "no row appears on two pages");

    // The currency filter is a separate axis.
    let usd = BillingPriceModel::list(&db, 100, 0, Some("USD"), None).await?;
    println!("USD rows: {}", usd.len());
    assert!(
        usd.iter().all(|p| p.currency == "USD"),
        "every row the currency filter returned is USD"
    );
    assert!(
        usd.iter().all(|p| p.model != "cny-model"),
        "and the CNY row is not among them"
    );

    // The region filter too, and the two together.
    let eu = BillingPriceModel::list(&db, 100, 0, None, Some("eu-west-1")).await?;
    assert_eq!(eu.len(), 1, "one row in eu-west-1");
    assert_eq!(eu[0].model, "eu-model");
    let both = BillingPriceModel::list(&db, 100, 0, Some("USD"), Some("eu-west-1")).await?;
    assert_eq!(both.len(), 1, "and the two filters together still find it");
    let contradictory =
        BillingPriceModel::list(&db, 100, 0, Some("CNY"), Some("eu-west-1")).await?;
    assert!(
        contradictory.is_empty(),
        "a combination with no rows returns none"
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn get_all_currencies_reports_the_currency_at_one_region() -> Result<(), Box<dyn Error>> {
    // **The name is longer than the behaviour.** `get_all_currencies(db, model, region)` filters by model **and
    // region** -- `WHERE model = ? AND region = ?` (`billing_price.rs:238`) -- and the key is `(model, region)`,
    // so it can only ever return **the one row** at that region, whatever the method is called.
    //
    // I had assumed it listed a model's currencies across regions; the first version of this test asserted two
    // entries for a model priced in two regions and measured one. This is the shape that exists.
    let (db, path) = fresh_db("all_currencies").await?;
    BillingPriceModel::upsert(&db, &price("one-currency", "USD", None, 1, 0)).await?;
    BillingPriceModel::upsert(&db, &price("two-regions", "USD", None, 1, 0)).await?;
    BillingPriceModel::upsert(&db, &price("two-regions", "CNY", Some("cn-north-1"), 2, 0)).await?;

    // At the universal region there is exactly one row, so exactly one entry.
    let universal = BillingPriceModel::get_all_currencies(&db, "two-regions", None).await?;
    println!(
        "two-regions at the universal region: {:?}",
        universal
            .iter()
            .map(|p| (&p.currency, &p.region))
            .collect::<Vec<_>>()
    );
    assert_eq!(universal.len(), 1, "one row at region \"\", so one entry");
    assert_eq!(universal[0].currency, "USD");

    // At the named region there is the other row.
    let named =
        BillingPriceModel::get_all_currencies(&db, "two-regions", Some("cn-north-1")).await?;
    println!(
        "two-regions at cn-north-1: {:?}",
        named
            .iter()
            .map(|p| (&p.currency, &p.region))
            .collect::<Vec<_>>()
    );
    assert_eq!(named.len(), 1, "one row at cn-north-1, so one entry");
    assert_eq!(named[0].currency, "CNY");

    // The two rows really are distinct in the table, which is what makes the region the axis rather than the
    // currency.
    let rows = rows_for(&db, "two-regions").await?;
    println!("rows for two-regions: {rows:?}");
    assert_eq!(rows.len(), 2, "two regions, two rows");
    assert!(
        rows.iter().any(|r| r.0 == "USD" && r.1.is_empty())
            && rows.iter().any(|r| r.0 == "CNY" && r.1 == "cn-north-1"),
        "one USD row at the empty region and one CNY row at cn-north-1"
    );

    // A model nobody has priced reports an empty list rather than failing.
    assert!(
        BillingPriceModel::get_all_currencies(&db, "unknown-model", None)
            .await?
            .is_empty(),
        "an unknown model reports no currencies"
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn a_missing_regional_price_falls_back_to_the_universal_one() -> Result<(), Box<dyn Error>> {
    // The other fallback, and the one the service layer's regional precedence is built on:
    // `get_by_model_region` tries `(model, region)` and then `(model, "")`, so a model priced once is priced in
    // every region until a region overrides it. Recorded here because it is a second place where a lookup
    // returns something other than what was asked for, and a caller needs to know that.
    let (db, path) = fresh_db("region_fallback").await?;
    BillingPriceModel::upsert(&db, &price("fallback", "USD", None, 1_000_000, 0)).await?;

    let universal = BillingPriceModel::get_by_model_region(&db, "fallback", None).await?;
    assert_eq!(
        universal.map(|p| p.input_price),
        Some(1_000_000),
        "the universal price is found directly"
    );

    let unnamed_region =
        BillingPriceModel::get_by_model_region(&db, "fallback", Some("ap-south-1")).await?;
    println!("ap-south-1 before an override exists: {unnamed_region:?}");
    assert_eq!(
        unnamed_region.map(|p| p.input_price),
        Some(1_000_000),
        "and an unpriced region falls back to it rather than reporting no price"
    );

    // Once the region has its own price, the override wins -- so the fallback is a fallback and not a shadow.
    BillingPriceModel::upsert(
        &db,
        &price("fallback", "USD", Some("ap-south-1"), 2_000_000, 0),
    )
    .await?;
    let overridden =
        BillingPriceModel::get_by_model_region(&db, "fallback", Some("ap-south-1")).await?;
    println!("ap-south-1 after an override: {overridden:?}");
    assert_eq!(
        overridden.map(|p| p.input_price),
        Some(2_000_000),
        "the regional price takes precedence over the universal one"
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn deleting_one_region_leaves_the_others_and_reports_its_count() -> Result<(), Box<dyn Error>>
{
    // Deletion is scoped to a model and a region, and returns a count. The count matters: a caller can tell
    // "removed the regional override" from "there was no override", which a `()` return could not express.
    let (db, path) = fresh_db("delete_scope").await?;
    BillingPriceModel::upsert(&db, &price("doomed", "USD", None, 1, 1)).await?;
    BillingPriceModel::upsert(&db, &price("doomed", "USD", Some("us-east-1"), 1, 1)).await?;
    BillingPriceModel::upsert(&db, &price("doomed", "USD", Some("eu-west-1"), 1, 1)).await?;
    BillingPriceModel::upsert(&db, &price("bystander", "USD", Some("us-east-1"), 2, 2)).await?;

    let removed = BillingPriceModel::delete_by_region(&db, "doomed", "us-east-1").await?;
    println!("delete_by_region removed {removed}");
    assert_eq!(removed, 1, "exactly the targeted row");

    assert!(
        BillingPriceModel::get(&db, "doomed", "USD", Some("us-east-1"))
            .await?
            .is_none(),
        "the targeted region is gone"
    );
    assert!(
        BillingPriceModel::get(&db, "doomed", "USD", None)
            .await?
            .is_some(),
        "the universal row survives"
    );
    assert!(
        BillingPriceModel::get(&db, "doomed", "USD", Some("eu-west-1"))
            .await?
            .is_some(),
        "and the other region"
    );
    assert!(
        BillingPriceModel::get(&db, "bystander", "USD", Some("us-east-1"))
            .await?
            .is_some(),
        "and another model in the same region"
    );

    // A region that was never there reports zero rather than failing, which is the distinction the count exists
    // for.
    let again = BillingPriceModel::delete_by_region(&db, "doomed", "us-east-1").await?;
    assert_eq!(again, 0, "a second delete reports nothing removed");

    BillingPriceModel::delete_all_for_model(&db, "doomed").await?;
    assert!(
        rows_for(&db, "doomed").await?.is_empty(),
        "delete_all_for_model removed the rest"
    );
    assert!(
        BillingPriceModel::get(&db, "bystander", "USD", Some("us-east-1"))
            .await?
            .is_some(),
        "and did not touch another model"
    );

    cleanup(db, path).await;
    Ok(())
}

// -------------------------------------------------------------------------------------------
// tiers
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn tiers_persist_in_order_and_can_be_replaced() -> Result<(), Box<dyn Error>> {
    // The plan's "tier 查询/更新". `upsert_tier` writes one tier at a time, so the tests are about the set: they
    // come back in order, `has_tiered_pricing` reports the set rather than a row, and `delete_tiers` clears the
    // model's tiers so a replacement set is not merged with the old one.
    let (db, path) = fresh_db("tiers").await?;

    let tier = |start: i64, end: Option<i64>, input: i64| TieredPriceInput {
        model: "tiered-model".to_string(),
        currency: Some("USD".to_string()),
        tier_type: Some("context_length".to_string()),
        region: None,
        tier_start: start,
        tier_end: end,
        input_price: input,
        output_price: input * 2,
    };

    assert!(
        !BillingTieredPriceModel::has_tiered_pricing(&db, "tiered-model").await?,
        "a model with no tiers reports false rather than failing"
    );

    for (start, end, input) in [
        (0_i64, Some(32_000_i64), 1_000_000_i64),
        (32_000, Some(128_000), 2_000_000),
        (128_000, None, 3_000_000),
    ] {
        BillingTieredPriceModel::upsert_tier(&db, &tier(start, end, input)).await?;
    }

    assert!(
        BillingTieredPriceModel::has_tiered_pricing(&db, "tiered-model").await?,
        "three tiers means the model is tiered"
    );

    let tiers = BillingTieredPriceModel::get_tiers(&db, "tiered-model", None).await?;
    println!(
        "tiers: {:?}",
        tiers
            .iter()
            .map(|t| (t.tier_start, t.tier_end))
            .collect::<Vec<_>>()
    );
    assert_eq!(tiers.len(), 3, "all three are stored");
    assert_eq!(
        tiers.iter().map(|t| t.tier_start).collect::<Vec<_>>(),
        vec![0, 32_000, 128_000],
        "and returned in ascending start order, which is the order a lookup needs"
    );

    // The 64-bit boundary on the open-ended tier, because `tier_end` is nullable and both bounds are `i64`.
    BillingTieredPriceModel::upsert_tier(
        &db,
        &TieredPriceInput {
            model: "wide-tier".to_string(),
            currency: Some("USD".to_string()),
            tier_type: Some("context_length".to_string()),
            region: None,
            tier_start: 0,
            tier_end: Some(i64::MAX),
            input_price: i64::MAX,
            output_price: i64::MIN,
        },
    )
    .await?;
    let wide = BillingTieredPriceModel::get_tiers(&db, "wide-tier", None).await?;
    assert_eq!(
        wide[0].tier_end,
        Some(i64::MAX),
        "the tier end round-trips at the limit"
    );
    assert_eq!(wide[0].input_price, i64::MAX, "and so does the price");
    assert_eq!(wide[0].output_price, i64::MIN, "including a negative one");

    // Replacing the set: delete, then write one, and the result must be exactly one tier rather than four.
    BillingTieredPriceModel::delete_tiers(&db, "tiered-model", None).await?;
    assert!(
        BillingTieredPriceModel::get_tiers(&db, "tiered-model", None)
            .await?
            .is_empty(),
        "delete_tiers cleared the model's tiers"
    );
    assert!(
        !BillingTieredPriceModel::has_tiered_pricing(&db, "tiered-model").await?,
        "and the model no longer reports as tiered"
    );

    BillingTieredPriceModel::upsert_tier(&db, &tier(0, None, 42)).await?;
    let replaced = BillingTieredPriceModel::get_tiers(&db, "tiered-model", None).await?;
    assert_eq!(
        replaced.len(),
        1,
        "the replacement set is exactly the one tier written"
    );
    assert_eq!(replaced[0].tier_end, None, "and its open end is preserved");

    cleanup(db, path).await;
    Ok(())
}

// -------------------------------------------------------------------------------------------
// dialect parity
// -------------------------------------------------------------------------------------------

#[test]
fn the_two_dialects_name_the_same_columns() {
    // The plan's "双后端字段一致". **There is no PostgreSQL server or container runtime on this machine**, so this
    // compares the two dialects' SQL column lists in the source rather than running both. That is a weaker claim
    // and it is stated as such: it catches a column added to one branch and not the other, and it cannot catch a
    // type, constraint or default difference.
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/billing_price.rs"))
        .expect("billing_price.rs is part of this crate");

    // Every statement in this module is written twice, once per dialect, inside `adapt_sql`. The parity claim is
    // that the two strings name the same columns -- so a column used in a statement appears at least twice in the
    // file (once per dialect), or not at all.
    for column in [
        "input_price",
        "output_price",
        "currency",
        "region",
        "model",
        "cache_read_input_price",
        "cache_creation_input_price",
        "batch_input_price",
        "batch_output_price",
        "updated_at",
    ] {
        let occurrences = src.matches(column).count();
        assert!(
            occurrences >= 2,
            "the column `{column}` appears {occurrences} time(s); a column that appears once is likely present \
             in only one dialect, and this parity claim is static rather than executed"
        );
    }

    // The placeholder helper must be used rather than a hard-coded `?` or `$n`, which is what lets one statement
    // serve both dialects.
    assert!(
        src.contains("adapt_sql") || src.contains("phs(") || src.contains("ph("),
        "the module must go through the placeholder helper for a single statement to serve both backends"
    );
    let hard_coded = src.matches("VALUES ($1").count() + src.matches("VALUES (?").count();
    println!("hard-coded placeholder prefixes found: {hard_coded}");
}
