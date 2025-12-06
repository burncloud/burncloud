use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_api_health() -> anyhow::Result<()> {
    let port = 4000;
    tokio::spawn(async move {
        if let Err(e) = burncloud_server::start_server(port).await {
            eprintln!("Server error: {}", e);
        }
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/api/channels", port);
    
    let resp = client.get(&url).send().await?;
    assert_eq!(resp.status(), 200);
    
    Ok(())
}
