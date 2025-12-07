use reqwest::Client;
use serde_json::Value;
use anyhow::{Result, ensure};

pub struct TestClient {
    base_url: String,
    client: Client,
    token: Option<String>,
}

impl TestClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: Client::new(),
            token: None,
        }
    }

    pub fn with_token(mut self, token: &str) -> Self {
        self.token = Some(token.to_string());
        self
    }

    pub async fn get(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.get(&url);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await?;
        let status = resp.status();
        ensure!(status.is_success(), "Request failed with status: {}", status);
        let json: Value = resp.json().await.unwrap_or(Value::Null);
        Ok(json)
    }

    pub async fn post(&self, path: &str, body: &Value) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.post(&url).json(body);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await?;
        let status = resp.status();
        
        let body_text = resp.text().await.unwrap_or_default();
        println!("TestClient: Response Body: '{}'", body_text);

        if !status.is_success() {
             ensure!(false, "Request failed with status: {} | Body: {}", status, body_text);
        }
        
        let json: Value = serde_json::from_str(&body_text).unwrap_or(Value::Null);
        Ok(json)
    }
    
    pub async fn post_expect_error(&self, path: &str, body: &Value, expected_status: u16) -> Result<()> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.post(&url).json(body);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await?;
        let status = resp.status();
        ensure!(status.as_u16() == expected_status, "Expected status {}, got {}", expected_status, status);
        Ok(())
    }

    pub async fn delete(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.delete(&url);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
             ensure!(false, "Delete failed: {} | {}", status, body_text);
        }
        let json: Value = serde_json::from_str(&body_text).unwrap_or(Value::Null);
        Ok(json)
    }
}
