//! Dynamic Protocol Adaptor Module
//!
//! This module provides a dynamic adaptor that can be configured at runtime
//! through protocol configurations stored in the database.

use crate::adaptor::factory::ChannelAdaptor;
use crate::adaptor::mapping::{apply_mapping, extract_value, RequestMapping, ResponseMapping};
use async_trait::async_trait;
use burncloud_common::types::OpenAIChatRequest;
use reqwest::RequestBuilder;
use serde_json::Value;

/// Dynamic adaptor that uses protocol configuration at runtime
pub struct DynamicAdaptor {
    /// Channel type identifier
    channel_type: i32,
    /// API version string
    api_version: String,
    /// Chat endpoint template (supports placeholders)
    chat_endpoint: Option<String>,
    /// Embed endpoint template
    embed_endpoint: Option<String>,
    /// Request mapping configuration
    request_mapping: Option<RequestMapping>,
    /// Response mapping configuration
    response_mapping: Option<ResponseMapping>,
}

impl DynamicAdaptor {
    /// Create a new dynamic adaptor with the given configuration
    pub fn new(
        channel_type: i32,
        api_version: String,
        chat_endpoint: Option<String>,
        embed_endpoint: Option<String>,
        request_mapping: Option<RequestMapping>,
        response_mapping: Option<ResponseMapping>,
    ) -> Self {
        Self {
            channel_type,
            api_version,
            chat_endpoint,
            embed_endpoint,
            request_mapping,
            response_mapping,
        }
    }

    /// Build endpoint URL with placeholder substitution
    pub fn build_endpoint(&self, base_url: &str, model: &str) -> String {
        let endpoint = self.chat_endpoint.as_deref().unwrap_or("/v1/chat/completions");

        // Replace placeholders
        let endpoint = endpoint.replace("{deployment_id}", model);
        let endpoint = endpoint.replace("{model}", model);

        // Combine with base URL
        format!("{}{}", base_url.trim_end_matches('/'), endpoint)
    }

    /// Parse request mapping from JSON string
    pub fn parse_request_mapping(json: &str) -> Option<RequestMapping> {
        serde_json::from_str(json).ok()
    }

    /// Parse response mapping from JSON string
    pub fn parse_response_mapping(json: &str) -> Option<ResponseMapping> {
        serde_json::from_str(json).ok()
    }
}

#[async_trait]
impl ChannelAdaptor for DynamicAdaptor {
    /// Returns the name of the adaptor
    fn name(&self) -> &'static str {
        "Dynamic"
    }

    /// Convert an OpenAI-format request to the target format
    fn convert_request(&self, req: &OpenAIChatRequest) -> Option<Value> {
        let mut json = serde_json::to_value(req).ok()?;

        // Apply request mapping if configured
        if let Some(ref mapping) = self.request_mapping {
            apply_mapping(&mut json, mapping);
        }

        Some(json)
    }

    /// Build the HTTP request for the target API
    async fn build_request(
        &self,
        _client: &reqwest::Client,
        builder: RequestBuilder,
        _api_key: &str,
        body: &Value,
    ) -> RequestBuilder {
        let mut body = body.clone();

        // Apply request mapping if configured
        if let Some(ref mapping) = self.request_mapping {
            apply_mapping(&mut body, mapping);
        }

        builder
            .header("Content-Type", "application/json")
            .json(&body)
    }

    /// Convert a response from the target API to OpenAI format
    fn convert_response(&self, resp: Value, _upstream_name: &str) -> Option<Value> {
        // If no response mapping, return as-is (assume OpenAI format)
        let response_mapping = self.response_mapping.as_ref()?;

        // Extract content using the configured path
        let content = response_mapping
            .content_path
            .as_ref()
            .and_then(|path| extract_value(&resp, path))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Extract usage if configured
        let usage = response_mapping
            .usage_path
            .as_ref()
            .and_then(|path| extract_value(&resp, path))
            .cloned();

        // Build OpenAI-format response
        Some(serde_json::json!({
            "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
            "object": "chat.completion",
            "created": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            "model": "dynamic",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content
                },
                "finish_reason": "stop"
            }],
            "usage": usage.unwrap_or(serde_json::json!({
                "prompt_tokens": 0,
                "completion_tokens": 0,
                "total_tokens": 0
            }))
        }))
    }

    /// Convert a streaming response chunk from the target API to OpenAI format
    fn convert_stream_response(&self, _chunk: &str) -> Option<String> {
        // Streaming conversion is complex and requires protocol-specific parsing
        // For now, we return the chunk as-is and rely on the token parser
        None
    }

    /// Check if streaming is supported
    fn supports_stream(&self) -> bool {
        true // Most APIs support streaming
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_adaptor_creation() {
        let adaptor = DynamicAdaptor::new(
            3, // Azure
            "2024-02-01".to_string(),
            Some("/deployments/{deployment_id}/chat/completions".to_string()),
            Some("/deployments/{deployment_id}/embeddings".to_string()),
            None,
            None,
        );

        assert_eq!(adaptor.channel_type, 3);
        assert_eq!(adaptor.api_version, "2024-02-01");
    }

    #[test]
    fn test_build_endpoint_with_placeholder() {
        let adaptor = DynamicAdaptor::new(
            3,
            "2024-02-01".to_string(),
            Some("/deployments/{deployment_id}/chat/completions".to_string()),
            None,
            None,
            None,
        );

        let endpoint = adaptor.build_endpoint("https://example.openai.azure.com", "gpt-4");
        assert_eq!(
            endpoint,
            "https://example.openai.azure.com/deployments/gpt-4/chat/completions"
        );
    }

    #[test]
    fn test_build_endpoint_default() {
        let adaptor = DynamicAdaptor::new(
            1,
            "default".to_string(),
            None,
            None,
            None,
            None,
        );

        let endpoint = adaptor.build_endpoint("https://api.openai.com", "gpt-4");
        assert_eq!(endpoint, "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn test_parse_request_mapping() {
        let json = r#"{"field_map": {"input": "messages"}, "rename": {"model": "deployment_id"}}"#;
        let mapping = DynamicAdaptor::parse_request_mapping(json);

        assert!(mapping.is_some());
        let mapping = mapping.unwrap();
        assert_eq!(mapping.field_map.get("input"), Some(&"messages".to_string()));
        assert_eq!(mapping.rename.get("model"), Some(&"deployment_id".to_string()));
    }

    #[test]
    fn test_parse_response_mapping() {
        let json = r#"{"content_path": "choices[0].message.content", "usage_path": "usage"}"#;
        let mapping = DynamicAdaptor::parse_response_mapping(json);

        assert!(mapping.is_some());
        let mapping = mapping.unwrap();
        assert_eq!(
            mapping.content_path,
            Some("choices[0].message.content".to_string())
        );
        assert_eq!(mapping.usage_path, Some("usage".to_string()));
    }

    #[test]
    fn test_convert_request_without_mapping() {
        let adaptor = DynamicAdaptor::new(1, "default".to_string(), None, None, None, None);

        let req = OpenAIChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![burncloud_common::types::OpenAIChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: false,
            extra: std::collections::HashMap::new(),
        };

        let converted = adaptor.convert_request(&req);
        assert!(converted.is_some());

        let json = converted.unwrap();
        assert_eq!(json.get("model").and_then(|v| v.as_str()), Some("gpt-4"));
    }

    #[test]
    fn test_convert_request_with_mapping() {
        let mapping = RequestMapping::new()
            .add_rename("model", "deployment_id")
            .add_fixed_field("api-version", serde_json::json!("2025-01-01"));

        let adaptor = DynamicAdaptor::new(
            3,
            "2025-01-01".to_string(),
            None,
            None,
            Some(mapping),
            None,
        );

        let req = OpenAIChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![burncloud_common::types::OpenAIChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            temperature: None,
            max_tokens: None,
            stream: false,
            extra: std::collections::HashMap::new(),
        };

        let converted = adaptor.convert_request(&req);
        assert!(converted.is_some());

        let json = converted.unwrap();
        // model should be renamed to deployment_id
        assert!(json.get("model").is_none());
        assert_eq!(
            json.get("deployment_id").and_then(|v| v.as_str()),
            Some("gpt-4")
        );
        // fixed field should be added
        assert_eq!(
            json.get("api-version").and_then(|v| v.as_str()),
            Some("2025-01-01")
        );
    }

    #[test]
    fn test_convert_response_without_mapping() {
        let adaptor = DynamicAdaptor::new(1, "default".to_string(), None, None, None, None);

        let resp = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello back!"
                }
            }]
        });

        // Without response mapping, returns None
        let converted = adaptor.convert_response(resp, "test");
        assert!(converted.is_none());
    }

    #[test]
    fn test_convert_response_with_mapping() {
        let mapping = ResponseMapping::new()
            .content_path("output.text")
            .usage_path("usage");

        let adaptor = DynamicAdaptor::new(
            4, // Gemini
            "v1".to_string(),
            None,
            None,
            None,
            Some(mapping),
        );

        let resp = serde_json::json!({
            "output": {
                "text": "Hello from Gemini!"
            },
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        });

        let converted = adaptor.convert_response(resp, "test");
        assert!(converted.is_some());

        let json = converted.unwrap();
        assert_eq!(
            json.get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str()),
            Some("Hello from Gemini!")
        );
        assert!(json.get("usage").is_some());
    }
}
