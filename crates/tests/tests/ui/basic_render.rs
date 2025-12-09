use crate::common;
use burncloud_tests::TestClient;

#[tokio::test]
async fn test_ui_endpoints_availability() {
    let base_url = common::spawn_app().await;
    let client = reqwest::Client::new();

    let endpoints = vec![
        "/",
        "/login",
        "/register",
        "/channels",
        "/models",
        "/settings",
    ];

    for endpoint in endpoints {
        let url = format!("{}{}", base_url, endpoint);
        let resp = client
            .get(&url)
            .send()
            .await
            .expect("Failed to send request");

        // Assert 200 OK
        assert!(
            resp.status().is_success(),
            "Endpoint {} returned status {}",
            endpoint,
            resp.status()
        );

        // Assert Content-Type is HTML
        let content_type = resp
            .headers()
            .get("content-type")
            .expect("No content-type header");
        assert!(
            content_type.to_str().unwrap().contains("text/html"),
            "Endpoint {} is not HTML",
            endpoint
        );

        // Assert body contains basic HTML structure (e.g. <title> or <div id="main">)
        let body = resp.text().await.expect("Failed to get body");
        assert!(
            body.contains("<!DOCTYPE html>"),
            "Endpoint {} missing DOCTYPE",
            endpoint
        );
        assert!(
            body.contains("BurnCloud"),
            "Endpoint {} missing Title",
            endpoint
        );
    }
}
