#![allow(
    clippy::unwrap_used,
    reason = "integration test: unwrap failures are the intended failure signal"
)]

use burncloud_database::{create_database_with_url, Database};
use burncloud_service_models::{ModelCapabilityInput, ModelCapabilityModel};

async fn fresh_db(tag: &str) -> (Database, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "bc_model_capability_{}_{}_{}.db",
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

async fn cleanup(db: Database, path: &std::path::Path) {
    db.close().await.ok();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    std::fs::remove_file(path)
        .unwrap_or_else(|e| panic!("test database {} was not removed ({e})", path.display()));
}

#[tokio::test]
async fn capability_upsert_round_trips_through_real_table() {
    let (db, path) = fresh_db("round_trip").await;

    let first = ModelCapabilityInput {
        model: "gpt-test".to_string(),
        context_window: Some(128_000),
        max_output_tokens: Some(8_192),
        supports_vision: true,
        supports_function_calling: true,
        input_price: Some(2.5),
        output_price: Some(10.0),
    };
    ModelCapabilityModel::upsert(&db, &first).await.unwrap();

    let stored = ModelCapabilityModel::get(&db, "gpt-test")
        .await
        .unwrap()
        .expect("capability row must exist");
    assert_eq!(stored.model, "gpt-test");
    assert_eq!(stored.context_window, Some(128_000));
    assert_eq!(stored.max_output_tokens, Some(8_192));
    assert!(stored.supports_vision);
    assert!(stored.supports_function_calling);
    assert_eq!(stored.input_price, Some(2.5));
    assert_eq!(stored.output_price, Some(10.0));
    assert!(stored.synced_at.is_some());

    let updated = ModelCapabilityInput {
        context_window: Some(200_000),
        supports_vision: false,
        input_price: Some(3.0),
        ..first
    };
    ModelCapabilityModel::upsert(&db, &updated).await.unwrap();

    let stored = ModelCapabilityModel::get(&db, "gpt-test")
        .await
        .unwrap()
        .expect("updated capability row must exist");
    assert_eq!(stored.context_window, Some(200_000));
    assert!(!stored.supports_vision);
    assert_eq!(stored.input_price, Some(3.0));

    cleanup(db, &path).await;
}
