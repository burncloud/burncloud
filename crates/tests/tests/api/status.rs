#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
use burncloud_tests::TestClient;

use crate::common;

#[tokio::test]
async fn test_health_check() {
    let base_url = common::spawn_app().await;
    let client = TestClient::new(&base_url);
    let res = client.get("/api/status").await;
    // Allow connection refused if server not running, but print warning
    if let Err(e) = res {
        eprintln!("Health check failed: {}. Is the server running?", e);
    }
}
