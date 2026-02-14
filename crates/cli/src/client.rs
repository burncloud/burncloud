use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Serialize, Deserialize, Default)]
pub struct ClientConfig {
    pub base_url: String,
    pub token: Option<String>,
}

pub struct ApiClient {
    client: reqwest::Client,
    config: ClientConfig,
    config_path: PathBuf,
}

impl ApiClient {
    pub async fn new() -> Result<Self> {
        let home_dir = dirs::home_dir().context("无法找到用户主目录")?;
        let config_dir = home_dir.join(".burncloud");
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).await?;
        }
        let config_path = config_dir.join("credentials.json");

        let config = if config_path.exists() {
            let content = fs::read_to_string(&config_path).await?;
            serde_json::from_str(&content).unwrap_or_else(|_| ClientConfig {
                base_url: "http://127.0.0.1:3000".to_string(),
                ..Default::default()
            })
        } else {
            ClientConfig {
                base_url: "http://127.0.0.1:3000".to_string(),
                ..Default::default()
            }
        };

        Ok(Self {
            client: reqwest::Client::new(),
            config,
            config_path,
        })
    }

    pub async fn save_config(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.config)?;
        fs::write(&self.config_path, content).await?;
        Ok(())
    }

    pub async fn login(&mut self, email: &str, password: &str) -> Result<String> {
        let url = format!("{}/api/auth/login", self.config.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "email": email,
                "password": password
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let error_text = resp.text().await?;
            return Err(anyhow::anyhow!("登录失败: {}", error_text));
        }

        let json: serde_json::Value = resp.json().await?;
        
        // 假设返回格式为 { "token": "..." } 或者 { "data": { "token": "..." } }
        // 根据实际 API 调整。这里假设标准 JWT 返回。
        let token = json["token"]
            .as_str()
            .or_else(|| json["data"]["token"].as_str())
            .context("响应中未找到 token")?
            .to_string();

        self.config.token = Some(token.clone());
        self.save_config().await?;

        Ok(token)
    }
    
    pub fn get_token(&self) -> Option<&String> {
        self.config.token.as_ref()
    }
    
    pub async fn chat_completions(&self, model: &str, messages: Vec<serde_json::Value>, stream: bool) -> Result<reqwest::Response> {
        let url = format!("{}/v1/chat/completions", self.config.base_url);
        let token = self.config.token.as_ref().context("未登录，请先运行 login 命令")?;
        
        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": stream
        });

        let resp = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;
            
        if !resp.status().is_success() {
             let error_text = resp.text().await?;
             return Err(anyhow::anyhow!("请求失败 ({}): {}", resp.status(), error_text));
        }
        
        Ok(resp)
    }
}
