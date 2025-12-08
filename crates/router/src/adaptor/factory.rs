use burncloud_common::types::{ChannelType, OpenAIChatRequest};
use serde_json::Value;
use reqwest::RequestBuilder;

/// Trait defining the behavior for a channel adaptor.
/// This mirrors the structure of New API's channel adapters.
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
    fn build_request(
        &self,
        builder: RequestBuilder, 
        api_key: &str, 
        body: &Value
    ) -> RequestBuilder;

    /// Checks if the adaptor supports streaming for the given model/request.
    #[allow(dead_code)]
    fn supports_stream(&self) -> bool {
        true
    }
    
    // TODO: Add stream handling method
}

// Implementations will go here or in submodules
pub struct OpenAIAdaptor;
impl ChannelAdaptor for OpenAIAdaptor {
    fn name(&self) -> &'static str { "OpenAI" }
    fn build_request(&self, builder: RequestBuilder, api_key: &str, body: &Value) -> RequestBuilder {
        builder.bearer_auth(api_key).json(body)
    }
}

pub struct AnthropicAdaptor;
impl ChannelAdaptor for AnthropicAdaptor {
    fn name(&self) -> &'static str { "Anthropic" }
    fn build_request(&self, builder: RequestBuilder, api_key: &str, body: &Value) -> RequestBuilder {
        // Conversion logic should happen before calling this or inside here if we pass OpenAIChatRequest
        // For now assume body is already converted if convert_request was called
        builder
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(body)
    }
    fn convert_request(&self, request: &OpenAIChatRequest) -> Option<Value> {
        Some(crate::adaptor::claude::ClaudeAdaptor::convert_request(request.clone()))
    }
    fn convert_response(&self, response: Value, model_name: &str) -> Option<Value> {
        Some(crate::adaptor::claude::ClaudeAdaptor::convert_response(response, model_name))
    }
}

pub struct GoogleGeminiAdaptor;
impl ChannelAdaptor for GoogleGeminiAdaptor {
    fn name(&self) -> &'static str { "GoogleGemini" }
    fn build_request(&self, builder: RequestBuilder, api_key: &str, body: &Value) -> RequestBuilder {
        builder
            .header("x-goog-api-key", api_key)
            .json(body)
    }
    fn convert_request(&self, request: &OpenAIChatRequest) -> Option<Value> {
        Some(crate::adaptor::gemini::GeminiAdaptor::convert_request(request.clone()))
    }
    fn convert_response(&self, response: Value, model_name: &str) -> Option<Value> {
        Some(crate::adaptor::gemini::GeminiAdaptor::convert_response(response, model_name))
    }
}

pub struct AdaptorFactory;

impl AdaptorFactory {
    pub fn get_adaptor(channel_type: ChannelType) -> Box<dyn ChannelAdaptor> {
        match channel_type {
            ChannelType::OpenAI | ChannelType::Azure | ChannelType::DeepSeek | ChannelType::Moonshot => Box::new(OpenAIAdaptor),
            ChannelType::Anthropic => Box::new(AnthropicAdaptor),
            ChannelType::Gemini | ChannelType::VertexAi => Box::new(GoogleGeminiAdaptor),
            // Add more mappings here
            _ => Box::new(OpenAIAdaptor), // Default to OpenAI-compatible
        }
    }
}
