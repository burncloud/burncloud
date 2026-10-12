#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! Cache read/write cost must be attributed to separate fields (#618).
//!
//! `compute_breakdown` has always computed the read and the write side separately, then
//! merged them into a single `cache_cost`. The router wrote that merged number into
//! `router_logs.cache_read_cost` and left `cache_write_cost` at 0, so:
//!
//! * the money was right — the merged figure is part of the total either way;
//! * but any per-column report was wrong, and cache **writes** (billed at a surcharge
//!   rather than a discount) were invisible as a category.
//!
//! The contract pinned here is the split plus the invariant that ties it to the old
//! single field:
//!
//! ```text
//! cache_read_cost  == read tokens  * read rate
//! cache_write_cost == write tokens * write rate
//! cache_cost       == cache_read_cost + cache_write_cost     (the old merged value)
//! total()          counts each cache token exactly once
//! ```
//!
//! Rates are set explicitly rather than through the fallbacks, so a field reading the
//! wrong rate shows up as a wrong number rather than as a coincidence.

use burncloud_commerce_billing::types::{CostBreakdown, UnifiedUsage};
use burncloud_commerce_billing::{BillingPriceModel, PriceInput};
use burncloud_commerce_billing::{CostCalculator, PriceCache};
use burncloud_database::{create_database_with_url, Database};
use std::error::Error;

/// One nano-dollar per token for input, so every other rate is a readable multiple.
const INPUT_PRICE: i64 = 1_000_000;
/// 10% of input — the documented cache-read discount.
const CACHE_READ_PRICE: i64 = 100_000;
/// 125% of input — the documented cache-write surcharge.
const CACHE_WRITE_PRICE: i64 = 1_250_000;

async fn fresh_db(tag: &str) -> Result<(Database, std::path::PathBuf), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!(
        "bc_cachecost_{}_{}_{}.db",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    if let Err(error) = std::fs::remove_file(&path) {
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err(error.into());
        }
    }
    let normalized = path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{}?mode=rwc", normalized);
    Ok((create_database_with_url(&url).await?, path))
}

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

fn priced_model(model: &str) -> PriceInput {
    PriceInput {
        model: model.to_string(),
        currency: "USD".to_string(),
        input_price: INPUT_PRICE,
        output_price: 2_000_000,
        cache_read_input_price: Some(CACHE_READ_PRICE),
        cache_creation_input_price: Some(CACHE_WRITE_PRICE),
        ..Default::default()
    }
}

async fn breakdown_for(tag: &str, usage: &UnifiedUsage) -> Result<CostBreakdown, Box<dyn Error>> {
    let (db, path) = fresh_db(tag).await?;
    BillingPriceModel::upsert(&db, &priced_model("cache-split")).await?;
    let cache = PriceCache::load(&db).await?;
    let calc = CostCalculator::new(cache);
    let result = calc
        .calculate("cache-split", usage, tag, false, false, None)
        .await?;
    let b = result.breakdown.clone();
    if result.usd_amount_nano != b.total() {
        return Err(std::io::Error::other(
            "the reported total must equal the sum of the components",
        )
        .into());
    }
    cleanup(db, path).await;
    Ok(b)
}

/// 1 000 read tokens at 0.1 nano each, 800 write tokens at 1.25 nano each.
fn read_and_write_usage() -> UnifiedUsage {
    UnifiedUsage {
        cache_read_tokens: 1_000,
        cache_write_tokens: 800,
        ..Default::default()
    }
}

#[tokio::test]
async fn read_and_write_costs_are_reported_separately_and_still_sum_to_the_merged_value() {
    let b = breakdown_for("both", &read_and_write_usage())
        .await
        .unwrap();
    println!(
        "cache_read_cost={} cache_write_cost={} cache_cost={} total={}",
        b.cache_read_cost,
        b.cache_write_cost,
        b.cache_cost,
        b.total()
    );

    assert_eq!(
        b.cache_read_cost, 100,
        "1_000 read tokens at 0.1 nano each (10% of the input rate)"
    );
    assert_eq!(
        b.cache_write_cost, 1_000,
        "800 write tokens at 1.25 nano each (125% of the input rate) — ten times the read charge for 20% \
         fewer tokens, which is exactly the asymmetry a merged column hides"
    );
    assert_eq!(
        b.cache_write_cost + b.cache_read_cost,
        b.cache_cost,
        "the retained merged figure must be the sum of the two, so no existing consumer of cache_cost \
         changes value"
    );
    assert_eq!(
        b.total(),
        1_100,
        "and the total counts each cache token once: 100 read + 1_000 write, not 2_200"
    );
}

#[tokio::test]
async fn a_read_only_request_reports_zero_write_cost() {
    let usage = UnifiedUsage {
        cache_read_tokens: 1_000,
        ..Default::default()
    };
    let b = breakdown_for("read_only", &usage).await.unwrap();
    println!("read only: {b:?}");

    assert_eq!(b.cache_read_cost, 100, "the read side");
    assert_eq!(
        b.cache_write_cost, 0,
        "nothing was written, so the write column must be zero rather than absent"
    );
    assert_eq!(b.cache_cost, 100, "and the merged figure is the read cost");
}

#[tokio::test]
async fn a_write_only_request_reports_zero_read_cost() {
    // The case that was previously mis-attributed: with no read tokens the old code put the
    // **write** cost into `cache_read_cost` and left `cache_write_cost` at 0.
    let usage = UnifiedUsage {
        cache_write_tokens: 800,
        ..Default::default()
    };
    let b = breakdown_for("write_only", &usage).await.unwrap();
    println!("write only: {b:?}");

    assert_eq!(
        b.cache_read_cost, 0,
        "nothing was read, so the read column must be zero — under the old write path this column held the \
         write cost"
    );
    assert_eq!(
        b.cache_write_cost, 1_000,
        "and the write cost lands in the write column"
    );
    assert_eq!(
        b.cache_cost, 1_000,
        "the merged figure is unchanged by the split, so the money is the same"
    );
}

#[tokio::test]
async fn a_request_with_no_cache_tokens_reports_zero_everywhere() {
    let usage = UnifiedUsage {
        input_tokens: 100,
        output_tokens: 100,
        ..Default::default()
    };
    let b = breakdown_for("no_cache", &usage).await.unwrap();

    assert_eq!(b.cache_read_cost, 0);
    assert_eq!(b.cache_write_cost, 0);
    assert_eq!(b.cache_cost, 0);
    assert_eq!(b.total(), 100 + 200, "input at 1 nano, output at 2 nano");
}
