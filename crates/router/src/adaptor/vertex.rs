use super::factory::ChannelAdaptor;
use super::gemini::GeminiAdaptor;
use burncloud_common::types::OpenAIChatRequest;
use reqwest::RequestBuilder;
use serde_json::Value;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use chrono::Utc;
use once_cell::sync::Lazy;
use dashmap::DashMap;
use async_trait::async_trait;

pub struct VertexAdaptor {
    pub auth_url: String,
}

impl Default for VertexAdaptor {
    fn default() -> Self {
        Self {
            auth_url: "https://oauth2.googleapis.com/token".to_string(),
        }
    }
}

#[derive(Deserialize)]
struct ServiceAccount {
    client_email: String,
    private_key: String,
    project_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    scope: String,
    aud: String,
    exp: i64,
    iat: i64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
    // other fields ignored
}

// Cache: client_email -> (access_token, expiration_timestamp)
static TOKEN_CACHE: Lazy<DashMap<String, (String, i64)>> = Lazy::new(DashMap::new);

#[async_trait]
impl ChannelAdaptor for VertexAdaptor {
    fn name(&self) -> &'static str {
        "VertexAi"
    }

    fn convert_stream_response(&self, chunk: &str) -> Option<String> {
        GeminiAdaptor::convert_stream_response(chunk)
    }

    async fn build_request(
        &self,
        client: &reqwest::Client,
        _builder: RequestBuilder,
        api_key: &str,
        body: &Value,
    ) -> RequestBuilder {
        // Parse Service Account
        let (client_email, private_key, sa_project_id) = match Self::parse_service_account(api_key) {
            Ok(acc) => acc,
            Err(e) => {
                eprintln!("VertexAdaptor: Failed to parse Service Account: {}", e);
                return client.post("http://invalid-service-account-config");
            }
        };

        // Get Access Token
        let token = match self.get_access_token(&client_email, &private_key).await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("VertexAdaptor: Failed to get Access Token: {}", e);
                return client.post("http://failed-to-get-token");
            }
        };

        // Parse Request Body to OpenAIChatRequest
        let openai_req: OpenAIChatRequest = match serde_json::from_value(body.clone()) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("VertexAdaptor: Failed to parse OpenAI Request: {}", e);
                return client.post("http://failed-to-parse-body");
            }
        };

        // Extract params
        let model = openai_req.model.clone();
        
        // Priority for project_id:
        // 1. Service Account Config
        // 2. Extra param "project_id"
        // 3. Default? No default, fail if missing.
        let project_id = openai_req.extra.get("project_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or(sa_project_id)
            .unwrap_or_default(); 

        let region = openai_req.extra.get("region")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "us-central1".to_string());

        // Convert Body
        let vertex_body = GeminiAdaptor::convert_request(openai_req);

        // Construct URL
        let url = format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:streamGenerateContent",
            region, project_id, region, model
        );

        // Create new request
        client.post(url)
            .bearer_auth(token)
            .json(&vertex_body)
    }
}

impl VertexAdaptor {
    #[allow(dead_code)] 
    fn parse_service_account(json_str: &str) -> Result<(String, String, Option<String>)> {
        let account: ServiceAccount = serde_json::from_str(json_str)?;
        Ok((account.client_email, account.private_key, account.project_id))
    }

    #[allow(dead_code)]
    pub async fn get_access_token(&self, client_email: &str, private_key: &str) -> Result<String> {
        let now = Utc::now().timestamp();

        // Check cache
        if let Some(entry) = TOKEN_CACHE.get(client_email) {
            let (token, exp) = entry.value();
            // Buffer 60s to avoid using an expiring token
            if *exp > now + 60 {
                return Ok(token.clone());
            }
        }

        // Create JWT
        let claims = Claims {
            iss: client_email.to_string(),
            scope: "https://www.googleapis.com/auth/cloud-platform".to_string(),
            aud: self.auth_url.clone(),
            exp: now + 3600,
            iat: now,
        };

        let header = Header::new(Algorithm::RS256);
        let key = EncodingKey::from_rsa_pem(private_key.as_bytes())
            .context("Failed to parse private key")?;
        let jwt = encode(&header, &claims, &key).context("Failed to sign JWT")?;

        // Request Token
        let client = reqwest::Client::new();
        let params = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &jwt),
        ];

        let res = client.post(&self.auth_url)
            .form(&params)
            .send()
            .await
            .context("Failed to send token request")?;
        
        if !res.status().is_success() {
             let status = res.status();
             let text = res.text().await.unwrap_or_default();
             anyhow::bail!("Failed to get token: {} - {}", status, text);
        }

        let token_res: TokenResponse = res.json().await.context("Failed to parse token response")?;
        
        // Update cache
        let exp_ts = now + token_res.expires_in;
        TOKEN_CACHE.insert(client_email.to_string(), (token_res.access_token.clone(), exp_ts));

        Ok(token_res.access_token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_service_account() {
        let json_str = r#"{
            "type": "service_account",
            "project_id": "test-project",
            "private_key_id": "12345",
            "private_key": "-----BEGIN PRIVATE KEY-----\nKEY\n-----END PRIVATE KEY-----\n",
            "client_email": "test@test-project.iam.gserviceaccount.com",
            "client_id": "123"
        }"#;

        let (email, key, project_id) = VertexAdaptor::parse_service_account(json_str).expect("Failed to parse");
        assert_eq!(email, "test@test-project.iam.gserviceaccount.com");
        assert_eq!(key, "-----BEGIN PRIVATE KEY-----\nKEY\n-----END PRIVATE KEY-----\n");
        assert_eq!(project_id, Some("test-project".to_string()));
    }

    #[tokio::test]
    async fn test_get_access_token() {
        let mut server = mockito::Server::new_async().await;
        let mock = server.mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token": "mock_token", "expires_in": 3600, "token_type": "Bearer"}"#)
            .create_async().await;

        let adaptor = VertexAdaptor {
            auth_url: server.url(),
        };

        let private_key = r#"-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQDaJKsOxgH3D2ah
v8vbh9n99AvHPOoIuJur/sV7tHZ9/bzMvnzVsQxxciagrVFve+XaE1mQjzNbRKB3
zsdpW2n1eUEtrO1PrQCA8BuaAnL/le4RryHyiDMy/hhGDVzvF55gSIgHv+aThDz7
/bK+GGJbLsoHOeFH7OV7wBWhNHpd5+I2GmAXEbCozPO9QjXkpsxOCbMu3lLQq99V
/F/HOWUbPMSIrVLKleL/yUhNO+0VUhYpWVAvO8gKP4hAf/qLvKNlV9zclBgANIhb
Fh8sC3OpvNVWiHNBoclVnmRB+OHJC51JBhHeZCJhM5IxDRtcYR4gGxcpNev8YFRF
xb76yA8dAgMBAAECggEAJ6D/MVsf2sPhu3M2I87Jd5TY+ewzQPvWjfel5Sv61bcd
kB1v3LtB/S8FXO23jFb4Afa/b99P713nf/Rg7h8k/+r0AAn4/584ZvQXs5IL1aol
WnGUK3T6RiJ6gullD2tdQnUSv0OprfVZRdcIHHgeEB4PJiJp7nDXHLTfyQ4ZR8sl
GstLN63/ZHNy4CyBdsjvJe0dqtJdXqK/ME6w5MtcHGpur8oSNqLsKKgIyXkcSasL
rhINjqIC1pN096a0nn9j9kYxJHas+JSu2gdhCuJ94t5B84B+Eb/7+MxmMLwygD0m
SbBA0MLfwzwmv6zsgLeXBxeK26AeeUTRjhXNVGSs8QKBgQD7alSesb/2Ecr8+2tm
UtzTY2wMKVcwNuYTLEmksIr43jIV1Gl73rMu3DH5hkXS1rOxBt6839QZDbWcL+7p
ruJpHW8o9/Qj7ELewqg8bKqXVvFTpqNQb13H0tQCrj1gQTopHwzBoThAvpVVxyZ2
s7FndVz+xsx53GXQnfisPb5YsQKBgQDeHwPepm3ABTGd1Qbp50ixEz/UtFL2F1Dy
jy4ylQR8ygqkgeE4NYh5WaubZnIKgn56cN2Rombv3LbqIe/N36Gj282k2rM4h7Km
1U1r1auIMZIon+zt1a2PlgmttUoAX5x2AuHI2DWE7ROmMTImV1SsW61qfg2xl1Nh
n/oyipf4LQKBgEtOKBZ4i1T7M1/fNuYpP7eZeg2SfGkWqIdppo1Ly/SLKVlcjFPr
+qO4lMd2rodeg+gsdJ8CNBdlAdbMjLU2Ct8NT/RngJsZ81Wh3J5sthQqmJJDwXsg
QGjP/2zmH8ArCW6zvDBrR9wsubI9uomnfSXOA5LUnP6LQ3vfNVLyE4ehAoGAe8kXE
/72DNwYIZh1iOb+6MgMe5Ke5UxrLTJEEaZgYNcMBU/oXrXev5oMe8ck6Nx+defu
Ytn5udTsDyEojjgB0dqOCUBkPq3JDxayVdU3CehuRruRg53gYrO/4xG0Eu81t8K1
Z4Oul8yzdZvXEez7YC6bP0zOftkRe8d23LHGLWUCgYAhW6lqcEmOqtw+TtQlVGQR
0K6nDeP5P0EnaG4ZiwVIiMpJhqj5avwlyDBeg9QdM+ubhqXHB5oCcLRLrP9PITf+
q/3tDxsxpwLbEpeg6nqaTxylV1V6Ky5oLq8u9tOsqP6eZ83STlGlPpimKH2FlO21
Apfww82b16AoK7qgtPcI8g==
-----END PRIVATE KEY-----"#;

        let token = adaptor.get_access_token("client@email.com", private_key).await.expect("Failed to get token");
        assert_eq!(token, "mock_token");
        
        mock.assert();
    }
    
    #[tokio::test]
    async fn test_build_request_conversion() {
        let mut server = mockito::Server::new_async().await;
        // Mock auth
        let mock = server.mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token": "mock_token", "expires_in": 3600, "token_type": "Bearer"}"#)
            .create_async().await;

        let adaptor = VertexAdaptor {
            auth_url: server.url(),
        };

        // Private Key for signing
        let private_key = r#"-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQDaJKsOxgH3D2ah
v8vbh9n99AvHPOoIuJur/sV7tHZ9/bzMvnzVsQxxciagrVFve+XaE1mQjzNbRKB3
zsdpW2n1eUEtrO1PrQCA8BuaAnL/le4RryHyiDMy/hhGDVzvF55gSIgHv+aThDz7
/bK+GGJbLsoHOeFH7OV7wBWhNHpd5+I2GmAXEbCozPO9QjXkpsxOCbMu3lLQq99V
/F/HOWUbPMSIrVLKleL/yUhNO+0VUhYpWVAvO8gKP4hAf/qLvKNlV9zclBgANIhb
Fh8sC3OpvNVWiHNBoclVnmRB+OHJC51JBhHeZCJhM5IxDRtcYR4gGxcpNev8YFRF
xb76yA8dAgMBAAECggEAJ6D/MVsf2sPhu3M2I87Jd5TY+ewzQPvWjfel5Sv61bcd
kB1v3LtB/S8FXO23jFb4Afa/b99P713nf/Rg7h8k/+r0AAn4/584ZvQXs5IL1aol
WnGUK3T6RiJ6gullD2tdQnUSv0OprfVZRdcIHHgeEB4PJiJp7nDXHLTfyQ4ZR8sl
GstLN63/ZHNy4CyBdsjvJe0dqtJdXqK/ME6w5MtcHGpur8oSNqLsKKgIyXkcSasL
rhINjqIC1pN096a0nn9j9kYxJHas+JSu2gdhCuJ94t5B84B+Eb/7+MxmMLwygD0m
SbBA0MLfwzwmv6zsgLeXBxeK26AeeUTRjhXNVGSs8QKBgQD7alSesb/2Ecr8+2tm
UtzTY2wMKVcwNuYTLEmksIr43jIV1Gl73rMu3DH5hkXS1rOxBt6839QZDbWcL+7p
ruJpHW8o9/Qj7ELewqg8bKqXVvFTpqNQb13H0tQCrj1gQTopHwzBoThAvpVVxyZ2
s7FndVz+xsx53GXQnfisPb5YsQKBgQDeHwPepm3ABTGd1Qbp50ixEz/UtFL2F1Dy
jy4ylQR8ygqkgeE4NYh5WaubZnIKgn56cN2Rombv3LbqIe/N36Gj282k2rM4h7Km
1U1r1auIMZIon+zt1a2PlgmttUoAX5x2AuHI2DWE7ROmMTImV1SsW61qfg2xl1Nh
n/oyipf4LQKBgEtOKBZ4i1T7M1/fNuYpP7eZeg2SfGkWqIdppo1Ly/SLKVlcjFPr
+qO4lMd2rodeg+gsdJ8CNBdlAdbMjLU2Ct8NT/RngJsZ81Wh3J5sthQqmJJDwXsg
QGjP/2zmH8ArCW6zvDBrR9wsubI9uomnfSXOA5LUnP6LQ3vfNVLyE4ehAoGAe8kXE
/72DNwYIZh1iOb+6MgMe5Ke5UxrLTJEEaZgYNcMBU/oXrXev5oMe8ck6Nx+defu
Ytn5udTsDyEojjgB0dqOCUBkPq3JDxayVdU3CehuRruRg53gYrO/4xG0Eu81t8K1
Z4Oul8yzdZvXEez7YC6bP0zOftkRe8d23LHGLWUCgYAhW6lqcEmOqtw+TtQlVGQR
0K6nDeP5P0EnaG4ZiwVIiMpJhqj5avwlyDBeg9QdM+ubhqXHB5oCcLRLrP9PITf+
q/3tDxsxpwLbEpeg6nqaTxylV1V6Ky5oLq8u9tOsqP6eZ83STlGlPpimKH2FlO21
Apfww82b16AoK7qgtPcI8g==
-----END PRIVATE KEY-----"#;

        let api_key_json = serde_json::json!({
            "client_email": "test@test-project.iam.gserviceaccount.com",
            "private_key": private_key,
            "project_id": "config-project" // Configured project
        }).to_string();
        
        let client = reqwest::Client::new();
        let builder = client.post("http://placeholder"); // dummy
        
        // OpenAI Style Body
        let body = serde_json::json!({
            "model": "gemini-pro",
            "messages": [
                { "role": "user", "content": "Hello Vertex" }
            ],
            // Extra params override
            "region": "asia-northeast1",
            "project_id": "override-project"
        });

        let req_builder = adaptor.build_request(&client, builder, &api_key_json, &body).await;
        let req = req_builder.build().expect("Failed to build req");

        // Verify URL: Should use "override-project" and "asia-northeast1"
        assert_eq!(req.url().as_str(), "https://asia-northeast1-aiplatform.googleapis.com/v1/projects/override-project/locations/asia-northeast1/publishers/google/models/gemini-pro:streamGenerateContent");
        
        // Verify Body: Should be converted to Gemini format
        
        if let Some(body) = req.body() {
             let bytes = body.as_bytes().unwrap();
             let json_body: serde_json::Value = serde_json::from_slice(bytes).unwrap();
             // Check if it has "contents" (Gemini) instead of "messages" (OpenAI)
             assert!(json_body.get("contents").is_some());
             assert!(json_body.get("messages").is_none());
             assert_eq!(json_body["contents"][0]["parts"][0]["text"], "Hello Vertex");
        } else {
            panic!("Request has no body");
        }

        mock.assert();
    }
}