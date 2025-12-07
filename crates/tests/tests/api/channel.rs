use burncloud_tests::TestClient;
use serde_json::json;
use uuid::Uuid;

use crate::common as common_mod;

#[tokio::test]
async fn test_channel_lifecycle() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());
    
    let channel_name = format!("Test Channel {}", Uuid::new_v4());
    
    // 1. Create
    let body = json!({
        "type": 1,
        "key": "sk-lifecycle-key",
        "name": channel_name,
        "base_url": "http://example.com",
        "models": "gpt-lifecycle",
        "group": "default",
        "weight": 10,
        "priority": 5
    });
    
    let res = client.post("/console/api/channel", &body).await.expect("Create failed");
    assert_eq!(res["success"], true);
    let id = res["data"]["id"].as_i64().expect("No ID returned");
    println!("Created channel ID: {}", id);
    
    // 2. Get
    let get_res = client.get(&format!("/console/api/channel/{}", id)).await.expect("Get failed");
    assert_eq!(get_res["success"], true);
    assert_eq!(get_res["data"]["name"], channel_name);
    
    // 3. Update
    let update_body = json!({
        "id": id,
        "type": 1,
        "key": "sk-lifecycle-key-updated",
        "name": channel_name,
        "base_url": "http://example.com",
        "models": "gpt-lifecycle-v2", 
        "group": "default",
        "weight": 20,
        "priority": 5
    });
    let update_res = client.put("/console/api/channel", &update_body).await.expect("Update failed");
    assert_eq!(update_res["success"], true);
    
    // 4. Verify Update via Get
    let get_res_2 = client.get(&format!("/console/api/channel/{}", id)).await.expect("Get 2 failed");
    assert_eq!(get_res_2["data"]["models"], "gpt-lifecycle-v2");
    
    // 5. List
    let list_res = client.get("/console/api/channel").await.expect("List failed");
    if list_res["success"] != true {
        println!("List failed: {}", list_res["message"]);
    }
    assert_eq!(list_res["success"], true);
    let channels = list_res["data"].as_array().expect("Data is not array");
    assert!(channels.iter().any(|c| c["id"].as_i64() == Some(id)));
    
    // 6. Delete
    let del_res = client.delete(&format!("/console/api/channel/{}", id)).await.expect("Delete failed");
    assert_eq!(del_res["success"], true);
    
    // 7. Get (404 or success: false)
    let get_res_3 = client.get(&format!("/console/api/channel/{}", id)).await.expect("Get 3 failed");
    assert_eq!(get_res_3["success"], false);
}
