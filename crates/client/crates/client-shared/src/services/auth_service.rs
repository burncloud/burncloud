use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub id: String,
    pub username: String,
    pub roles: Vec<String>,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResult {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<LoginResponse>,
}

pub struct AuthService;

impl AuthService {
    pub async fn login(username: &str, password: &str) -> Result<LoginResponse, String> {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let url = format!("http://127.0.0.1:{}/console/api/user/login", port);

        println!(
            "AuthService: Logging in {} with password length {}",
            username,
            password.len()
        );

        let body = serde_json::json!({
            "username": username,
            "password": password
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let json: AuthResult = resp.json().await.map_err(|e| e.to_string())?;
        println!("AuthService: Response: {:?}", json);

        if json.success {
            if let Some(data) = json.data {
                // TODO: Save token to LocalStorage (if web) or Memory (if desktop)
                // For now just return it
                Ok(data)
            } else {
                Err("No data received".to_string())
            }
        } else {
            Err(json.message.unwrap_or_else(|| "Login failed".to_string()))
        }
    }

    pub async fn register(
        username: &str,
        password: &str,
        email: Option<&str>,
    ) -> Result<LoginResponse, String> {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let url = format!("http://127.0.0.1:{}/console/api/user/register", port);

        let body = serde_json::json!({
            "username": username,
            "password": password,
            "email": email
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("网络错误: {}", e))?;

        let json: AuthResult = resp.json().await.map_err(|e| format!("响应解析错误: {}", e))?;

        if json.success {
            if let Some(data) = json.data {
                Ok(data)
            } else {
                Err("注册成功但未返回用户数据".to_string())
            }
        } else {
            Err(json["message"]
                .as_str()
                .unwrap_or("注册失败")
                .to_string())
        }
    }

    pub async fn check_username_availability(username: &str) -> Result<bool, String> {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let url = format!("http://127.0.0.1:{}/console/api/user/check_username?username={}", port, username);

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

        if json["success"].as_bool().unwrap_or(false) {
            Ok(json["data"]["available"].as_bool().unwrap_or(false))
        } else {
            Err(json["message"]
                .as_str()
                .unwrap_or("检查失败")
                .to_string())
        }
    }
}
