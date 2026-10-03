#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Real database mapping for the persistence rows (maintainer review of #602, edit 2).
//!
//! The in-crate parity tests prove the Rust-to-Rust conversions are lossless, but they cannot prove
//! that sqlx can actually decode the columns: `FromRow` derives, column names, SQLite type affinity
//! and the `#[sqlx(default)]` NULL handling are only exercised against a database.
//!
//! These tests therefore drive the real path, end to end:
//!
//! ```text
//! SQLite file  ->  migrations create billing_* tables
//!              ->  BillingPriceModel::upsert writes through the real INSERT
//!              ->  BillingPriceModel::get runs the real SELECT ... sqlx::query_as::<_, PriceRow>
//!              ->  PriceRow -> Price conversion
//!              ->  assert the domain values byte-for-byte
//! ```
//!
//! The row structs are crate-private, so nothing here can reach them directly -- which is the point:
//! the tests observe exactly what an outside caller observes.

use burncloud_commerce_contracts::pricing::{PriceInput, TieredPriceInput};
use burncloud_database::{create_database_with_url, Database};
use burncloud_database_billing::{BillingPriceModel, BillingTieredPriceModel};

/// Build a fresh SQLite database in a temporary file and run the real migrations.
///
/// A file (not `:memory:`) is used because the pool opens several connections and an in-memory
/// SQLite database is per-connection, which would make the migrations and the assertions see
/// different databases.
async fn fresh_db(tag: &str) -> (Database, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "bc_billing_{}_{}_{}.db",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&path);
    // Three slashes: absolute Windows paths need `sqlite:///C:/...` (see Database::new).
    let normalized = path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{}?mode=rwc", normalized);
    let db = create_database_with_url(&url)
        .await
        .unwrap_or_else(|e| panic!("open {url}: {e}"));
    (db, path)
}

fn cleanup(db: Database, path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    drop(db);
}

/// A price input with every column populated by a distinct value.
fn full_price_input() -> PriceInput {
    PriceInput {
        model: "gpt-4o".to_string(),
        currency: "USD".to_string(),
        input_price: 2_500_000_000,
        output_price: 10_000_000_000,
        cache_read_input_price: Some(250_000_000),
        cache_creation_input_price: Some(3_125_000_000),
        batch_input_price: Some(1_250_000_000),
        batch_output_price: Some(5_000_000_000),
        priority_input_price: Some(4_250_000_000),
        priority_output_price: Some(17_000_000_000),
        audio_input_price: Some(40_000_000_000),
        audio_output_price: Some(80_000_000_000),
        reasoning_price: Some(10_000_000_000),
        embedding_price: Some(20_000_000),
        image_price: Some(1_000_000),
        video_price: Some(2_000_000),
        music_price: Some(50_000_000),
        source: Some("litellm".to_string()),
        region: Some("international".to_string()),
        context_window: Some(128_000),
        max_output_tokens: Some(16_384),
        supports_vision: Some(true),
        supports_function_calling: Some(false),
        voices_pricing: Some(r#"{"alloy":15000000000}"#.to_string()),
        video_pricing: Some(r#"{"720p":50000000}"#.to_string()),
        asr_pricing: Some(r#"{"per_minute":360000000}"#.to_string()),
        realtime_pricing: Some(r#"{"audio_input":32000000000}"#.to_string()),
        model_type: Some("chat".to_string()),
    }
}

#[test]
fn migrations_create_the_billing_tables() {
    // Guards the rest of this file: if a future migration renames a table, the failures below must
    // point at the schema rather than at the mapping.
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let (db, path) = fresh_db("schema").await;
        let names: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name LIKE 'billing_%' ORDER BY name",
        )
        .fetch_all(db.get_connection().unwrap().pool())
        .await
        .unwrap();
        let names: Vec<String> = names.into_iter().map(|(n,)| n).collect();
        for expected in [
            "billing_prices",
            "billing_tiered_prices",
            "billing_exchange_rates",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "migration did not create {expected}; found {names:?}"
            );
        }
        cleanup(db, &path);
    });
}

#[tokio::test]
async fn price_survives_a_real_insert_and_select() {
    let (db, path) = fresh_db("price").await;
    let input = full_price_input();

    BillingPriceModel::upsert(&db, &input)
        .await
        .expect("real INSERT into billing_prices");

    // The real SELECT from billing_price.rs runs here: it decodes into the crate-private PriceRow
    // through sqlx::FromRow and converts to the Commerce domain type at the boundary.
    let stored = BillingPriceModel::get(&db, "gpt-4o", "USD", Some("international"))
        .await
        .expect("real SELECT from billing_prices")
        .expect("the row that was just inserted must be found");

    assert_eq!(stored.model, input.model);
    assert_eq!(stored.currency, input.currency);
    assert_eq!(stored.input_price, 2_500_000_000);
    assert_eq!(stored.output_price, 10_000_000_000);
    assert_eq!(stored.cache_read_input_price, Some(250_000_000));
    assert_eq!(stored.cache_creation_input_price, Some(3_125_000_000));
    assert_eq!(stored.batch_input_price, Some(1_250_000_000));
    assert_eq!(stored.batch_output_price, Some(5_000_000_000));
    assert_eq!(stored.priority_input_price, Some(4_250_000_000));
    assert_eq!(stored.priority_output_price, Some(17_000_000_000));
    assert_eq!(stored.audio_input_price, Some(40_000_000_000));
    assert_eq!(stored.audio_output_price, Some(80_000_000_000));
    assert_eq!(stored.reasoning_price, Some(10_000_000_000));
    assert_eq!(stored.embedding_price, Some(20_000_000));
    assert_eq!(stored.image_price, Some(1_000_000));
    assert_eq!(stored.video_price, Some(2_000_000));
    assert_eq!(stored.music_price, Some(50_000_000));
    assert_eq!(stored.source.as_deref(), Some("litellm"));
    assert_eq!(stored.region.as_deref(), Some("international"));
    assert_eq!(stored.context_window, Some(128_000));
    assert_eq!(stored.max_output_tokens, Some(16_384));
    assert_eq!(stored.model_type.as_deref(), Some("chat"));
    assert_eq!(
        stored.voices_pricing.as_deref(),
        Some(r#"{"alloy":15000000000}"#)
    );
    assert_eq!(
        stored.video_pricing.as_deref(),
        Some(r#"{"720p":50000000}"#)
    );
    // The INTEGER 0/1 columns must survive the round trip through `Option<i32>`.
    assert_eq!(stored.supports_vision_bool(), Some(true));
    assert_eq!(stored.supports_function_calling_bool(), Some(false));
    assert!(stored.id > 0, "SQLite assigned a row id");

    cleanup(db, &path);
}

#[tokio::test]
async fn absent_optional_columns_decode_as_none() {
    // Inserts only the required columns. What comes back depends on the column definition, and the
    // two cases are different -- so both are asserted here rather than assumed:
    //   * `supports_vision` / `supports_function_calling` are declared `INTEGER DEFAULT 0`, so the
    //     INSERT stores 0 and the read returns Some(0), not NULL.
    //   * columns without a DEFAULT stay NULL and must decode as None, which is the case the
    //     `#[sqlx(default)]` attributes on the row structs exist to tolerate.
    let (db, path) = fresh_db("defaults").await;
    db.execute_query(
        "INSERT INTO billing_prices (model, currency, input_price, output_price, region) \
         VALUES ('sparse-model', 'CNY', 1000000000, 2000000000, '')",
    )
    .await
    .unwrap();

    let stored = BillingPriceModel::get(&db, "sparse-model", "CNY", None)
        .await
        .expect("SELECT must succeed even when optional columns are NULL")
        .expect("sparse row must be found");

    assert_eq!(stored.input_price, 1_000_000_000);
    assert_eq!(stored.output_price, 2_000_000_000);

    // DEFAULT 0 columns: 0, not NULL.
    assert_eq!(stored.supports_vision, Some(0));
    assert_eq!(stored.supports_function_calling, Some(0));
    assert_eq!(stored.supports_vision_bool(), Some(false));

    // Columns without a DEFAULT: genuinely NULL.
    assert_eq!(stored.cache_read_input_price, None);
    assert_eq!(stored.voices_pricing, None);
    assert_eq!(stored.model_type, None);
    assert_eq!(stored.context_window, None);
    assert_eq!(stored.source, None);
    // `region` was written as the empty string by the INSERT above.
    assert_eq!(stored.region.as_deref(), Some(""));

    cleanup(db, &path);
}

#[tokio::test]
async fn tiered_price_rows_survive_a_real_insert_and_select() {
    let (db, path) = fresh_db("tiered").await;

    let tiers = [
        (
            0_i64,
            Some(32_000_i64),
            1_200_000_000_i64,
            2_400_000_000_i64,
        ),
        (32_000, Some(128_000), 2_400_000_000, 4_800_000_000),
        (128_000, None, 3_000_000_000, 6_000_000_000),
    ];
    for (start, end, input, output) in tiers {
        BillingTieredPriceModel::upsert_tier(
            &db,
            &TieredPriceInput {
                model: "qwen-max".to_string(),
                region: Some("cn".to_string()),
                currency: Some("CNY".to_string()),
                tier_type: Some("context_length".to_string()),
                tier_start: start,
                tier_end: end,
                input_price: input,
                output_price: output,
            },
        )
        .await
        .expect("real INSERT into billing_tiered_prices");
    }

    let stored = BillingTieredPriceModel::get_tiers(&db, "qwen-max", Some("cn"))
        .await
        .expect("real SELECT from billing_tiered_prices");

    assert_eq!(stored.len(), 3, "all three tiers must come back");
    // ORDER BY tier_start ASC is part of the production query.
    assert_eq!(stored[0].tier_start, 0);
    assert_eq!(stored[0].tier_end, Some(32_000));
    assert_eq!(stored[1].tier_start, 32_000);
    assert_eq!(stored[1].tier_end, Some(128_000));
    assert_eq!(stored[2].tier_start, 128_000);
    assert_eq!(
        stored[2].tier_end, None,
        "open-ended tier keeps its NULL bound"
    );
    assert_eq!(stored[2].input_price, 3_000_000_000);
    assert_eq!(stored[2].output_price, 6_000_000_000);
    assert_eq!(stored[0].currency.as_deref(), Some("CNY"));
    assert_eq!(stored[0].tier_type.as_deref(), Some("context_length"));
    assert_eq!(stored[0].model, "qwen-max");
    assert_eq!(stored[0].region.as_deref(), Some("cn"));
    assert!(stored[0].id > 0, "SQLite assigned a row id");

    cleanup(db, &path);
}
