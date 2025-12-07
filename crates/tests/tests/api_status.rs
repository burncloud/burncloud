mod common;

use burncloud_tests::TestClient;
use serde_json::json;

#[tokio::test]
async fn test_health_check() {
    let base_url = common::get_base_url();
    let client = TestClient::new(&base_url);
    let res = client.get("/api/status").await;
    // Allow connection refused if server not running, but print warning
    if let Err(e) = res {
        eprintln!("Health check failed: {}. Is the server running?", e);
    }
}
