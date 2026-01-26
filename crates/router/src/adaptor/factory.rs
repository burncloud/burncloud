use burncloud_common::types::{ChannelType, OpenAIChatRequest};
use reqwest::RequestBuilder;
use serde_json::Value;
use async_trait::async_trait;

/// Trait defining the behavior for a channel adaptor.
/// This mirrors the structure of New API's channel adapters.
#[async_trait]
pub trait ChannelAdaptor: Send + Sync {
    /// Returns the name of the adaptor (e.g., "OpenAI", "Claude").
    #[allow(dead_code)]
    fn name(&self) -> &'static str;

    /// Converts an OpenAI-compatible request to the vendor-specific request body.
    /// Returns `None` if the adaptor performs direct passthrough or modifies the request in `build_request` directly.
    fn convert_request(&self, _request: &OpenAIChatRequest) -> Option<Value> {
        None
    }

    /// Converts a vendor-specific response body back to an OpenAI-compatible response body.
    /// This is used for non-streaming responses.
    fn convert_response(&self, _response: Value, _model_name: &str) -> Option<Value> {
        None
    }

    /// Modifies the HTTP request builder before sending (e.g., setting headers, URL, body).
    /// This gives adaptors full control over how the request is sent.
    async fn build_request(&self, client: &reqwest::Client, builder: RequestBuilder, api_key: &str, body: &Value)
        -> RequestBuilder;

    /// Checks if the adaptor supports streaming for the given model/request.
    #[allow(dead_code)]
    fn supports_stream(&self) -> bool {
        true
    }

    /// Converts a vendor-specific stream chunk to an OpenAI-compatible SSE event string.
    /// Returns `None` if the adaptor performs direct passthrough.
    fn convert_stream_response(&self, _chunk: &str) -> Option<String> {
        None
    }
}

// Implementations will go here or in submodules
pub struct OpenAIAdaptor;
#[async_trait]
impl ChannelAdaptor for OpenAIAdaptor {
    fn name(&self) -> &'static str {
        "OpenAI"
    }
    async fn build_request(
        &self,
        _client: &reqwest::Client,
        builder: RequestBuilder,
        api_key: &str,
        body: &Value,
    ) -> RequestBuilder {
        builder.bearer_auth(api_key).json(body)
    }
}

pub struct AnthropicAdaptor;
#[async_trait]
impl ChannelAdaptor for AnthropicAdaptor {
    fn name(&self) -> &'static str {
        "Anthropic"
    }
    async fn build_request(
        &self,
        _client: &reqwest::Client,
        builder: RequestBuilder,
        api_key: &str,
        body: &Value,
    ) -> RequestBuilder {
        // Conversion logic should happen before calling this or inside here if we pass OpenAIChatRequest
        // For now assume body is already converted if convert_request was called
        builder
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(body)
    }
    fn convert_request(&self, request: &OpenAIChatRequest) -> Option<Value> {
        Some(crate::adaptor::claude::ClaudeAdaptor::convert_request(
            request.clone(),
        ))
    }
    fn convert_response(&self, response: Value, model_name: &str) -> Option<Value> {
        Some(crate::adaptor::claude::ClaudeAdaptor::convert_response(
            response, model_name,
        ))
    }
}

pub struct GoogleGeminiAdaptor;
#[async_trait]
impl ChannelAdaptor for GoogleGeminiAdaptor {
    fn name(&self) -> &'static str {
        "GoogleGemini"
    }
    async fn build_request(
        &self,
        _client: &reqwest::Client,
        builder: RequestBuilder,
        api_key: &str,
        body: &Value,
    ) -> RequestBuilder {
        builder.header("x-goog-api-key", api_key).json(body)
    }
    fn convert_request(&self, request: &OpenAIChatRequest) -> Option<Value> {
        Some(crate::adaptor::gemini::GeminiAdaptor::convert_request(
            request.clone(),
        ))
    }
    fn convert_response(&self, response: Value, model_name: &str) -> Option<Value> {
        Some(crate::adaptor::gemini::GeminiAdaptor::convert_response(
            response, model_name,
        ))
    }

    fn convert_stream_response(&self, chunk: &str) -> Option<String> {
        crate::adaptor::gemini::GeminiAdaptor::convert_stream_response(chunk)
    }
}

pub struct AdaptorFactory;

impl AdaptorFactory {
    pub fn get_adaptor(channel_type: ChannelType) -> Box<dyn ChannelAdaptor> {
        match channel_type {
            ChannelType::OpenAI
            | ChannelType::Azure
            | ChannelType::DeepSeek
            | ChannelType::Moonshot => Box::new(OpenAIAdaptor),
            ChannelType::Anthropic => Box::new(AnthropicAdaptor),
            ChannelType::Gemini => Box::new(GoogleGeminiAdaptor),
            ChannelType::VertexAi => Box::new(crate::adaptor::vertex::VertexAdaptor::default()),
            // Add more mappings here
            _ => Box::new(OpenAIAdaptor), // Default to OpenAI-compatible
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_adaptor() {
        let adaptor = AdaptorFactory::get_adaptor(ChannelType::VertexAi);
        assert_eq!(adaptor.name(), "VertexAi");

        let adaptor = AdaptorFactory::get_adaptor(ChannelType::Gemini);
        assert_eq!(adaptor.name(), "GoogleGemini");
        
        let adaptor = AdaptorFactory::get_adaptor(ChannelType::OpenAI);
        assert_eq!(adaptor.name(), "OpenAI");
    }
}