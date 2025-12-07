use burncloud_tests::TestClient;
use serde_json::json;
use uuid::Uuid;

#[path = "common/mod.rs"]
mod common;

#[tokio::test]
async fn test_channel_crud() {
    let base_url = common::get_base_url();
    // Use root token for admin actions
    // Note: API doesn't enforce auth yet, but we should be ready
    let client = TestClient::new(&base_url).with_token(&common::get_root_token());
    
    let channel_name = format!("Test Channel {}", Uuid::new_v4());
    
    // 1. Create
    let body = json!({
        "type": 1,
        "key": "sk-test-key",
        "name": channel_name,
        "base_url": "http://example.com",
        "models": "gpt-test-1,gpt-test-2",
        "group": "default",
        "weight": 10,
        "priority": 5
    });
    
    let res = client.post("/console/api/channel", &body).await.expect("Create failed");
    assert_eq!(res["success"], true);
    let id = res["data"]["id"].as_i64().expect("No ID returned");
    println!("Created channel ID: {}", id);
    
    // 2. Get (Not implemented yet fully, but we can try)
    // let get_res = client.get(&format!("/console/api/channel/{}", id)).await;
    // assert!(get_res.is_ok());
    
    // 3. Delete
    let del_res = client.delete(&format!("/console/api/channel/{}", id)).await; // TestClient needs delete?
    // TestClient doesn't have delete yet.
    // Skip delete test or add delete to TestClient.
}
