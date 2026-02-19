use async_trait::async_trait;
use burncloud_common::types::{ChannelType, OpenAIChatRequest};
use burncloud_database::Database;
use burncloud_database_models::{ProtocolConfig, ProtocolConfigModel};
use dashmap::DashMap;
use reqwest::RequestBuilder;
use serde_json::Value;
use std::sync::Arc;

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
    async fn build_request(
        &self,
        client: &reqwest::Client,
        builder: RequestBuilder,
        api_key: &str,
        body: &Value,
    ) -> RequestBuilder;

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
        client: &reqwest::Client,
        _builder: RequestBuilder,
        api_key: &str,
        body: &Value,
    ) -> RequestBuilder {
        // Extract model name from body
        let model = body
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("gemini-2.0-flash");

        // Convert OpenAI request to Gemini format
        let openai_req: Option<OpenAIChatRequest> = serde_json::from_value(body.clone()).ok();
        let gemini_body = if let Some(req) = openai_req {
            crate::adaptor::gemini::GeminiAdaptor::convert_request(req)
        } else {
            body.clone()
        };

        // Construct correct Gemini API URL: /v1beta/models/{model}:generateContent
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            model
        );

        client
            .post(&url)
            .header("x-goog-api-key", api_key)
            .json(&gemini_body)
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

/// Cache key for dynamic adaptor lookup
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct AdaptorCacheKey {
    channel_type: i32,
    api_version: String,
}

/// Factory that supports both static and dynamic adaptors with caching
pub struct DynamicAdaptorFactory {
    /// Database connection for loading protocol configs
    db: Arc<Database>,
    /// Cache of dynamic adaptors by channel_type + api_version
    cache: DashMap<AdaptorCacheKey, Arc<dyn ChannelAdaptor>>,
}

impl DynamicAdaptorFactory {
    /// Create a new DynamicAdaptorFactory with database connection
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            cache: DashMap::new(),
        }
    }

    /// Get an adaptor for the given channel type and API version
    /// Falls back to static adaptors for well-known channel types
    pub async fn get_adaptor(
        &self,
        channel_type: ChannelType,
        api_version: Option<&str>,
    ) -> Arc<dyn ChannelAdaptor> {
        let channel_type_id = channel_type as i32;
        let api_version = api_version.unwrap_or("default");

        // Check cache first
        let cache_key = AdaptorCacheKey {
            channel_type: channel_type_id,
            api_version: api_version.to_string(),
        };

        if let Some(adaptor) = self.cache.get(&cache_key) {
            return Arc::clone(&adaptor);
        }

        // Try to load dynamic config from database
        match self
            .load_dynamic_adaptor(channel_type_id, api_version)
            .await
        {
            Ok(Some(adaptor)) => {
                self.cache.insert(cache_key, Arc::clone(&adaptor));
                adaptor
            }
            Ok(None) => {
                // No dynamic config found, use static adaptor
                Arc::from(AdaptorFactory::get_adaptor(channel_type))
            }
            Err(e) => {
                eprintln!(
                    "Failed to load dynamic adaptor: {}, falling back to static",
                    e
                );
                Arc::from(AdaptorFactory::get_adaptor(channel_type))
            }
        }
    }

    /// Get an adaptor using the default API version for the channel type
    pub async fn get_default_adaptor(&self, channel_type: ChannelType) -> Arc<dyn ChannelAdaptor> {
        let channel_type_id = channel_type as i32;

        // Try to get default config from database
        match self.load_default_adaptor(channel_type_id).await {
            Ok(Some(adaptor)) => adaptor,
            Ok(None) => Arc::from(AdaptorFactory::get_adaptor(channel_type)),
            Err(e) => {
                eprintln!(
                    "Failed to load default adaptor: {}, falling back to static",
                    e
                );
                Arc::from(AdaptorFactory::get_adaptor(channel_type))
            }
        }
    }

    /// Load a dynamic adaptor from the database
    async fn load_dynamic_adaptor(
        &self,
        channel_type: i32,
        api_version: &str,
    ) -> anyhow::Result<Option<Arc<dyn ChannelAdaptor>>> {
        let config =
            ProtocolConfigModel::get_by_type_version(&self.db, channel_type, api_version).await?;

        match config {
            Some(protocol_config) => {
                let adaptor = self.create_dynamic_adaptor(protocol_config)?;
                Ok(Some(adaptor))
            }
            None => Ok(None),
        }
    }

    /// Load the default dynamic adaptor for a channel type
    async fn load_default_adaptor(
        &self,
        channel_type: i32,
    ) -> anyhow::Result<Option<Arc<dyn ChannelAdaptor>>> {
        let config = ProtocolConfigModel::get_default(&self.db, channel_type).await?;

        match config {
            Some(protocol_config) => {
                let adaptor = self.create_dynamic_adaptor(protocol_config)?;
                Ok(Some(adaptor))
            }
            None => Ok(None),
        }
    }

    /// Create a DynamicAdaptor from a ProtocolConfig
    fn create_dynamic_adaptor(
        &self,
        config: ProtocolConfig,
    ) -> anyhow::Result<Arc<dyn ChannelAdaptor>> {
        // Parse request and response mappings
        let request_mapping = config
            .request_mapping
            .as_ref()
            .and_then(|json| crate::adaptor::dynamic::DynamicAdaptor::parse_request_mapping(json));

        let response_mapping = config
            .response_mapping
            .as_ref()
            .and_then(|json| crate::adaptor::dynamic::DynamicAdaptor::parse_response_mapping(json));

        let adaptor = crate::adaptor::dynamic::DynamicAdaptor::new(
            config.channel_type,
            config.api_version,
            config.chat_endpoint,
            config.embed_endpoint,
            request_mapping,
            response_mapping,
        );

        Ok(Arc::new(adaptor))
    }

    /// Clear the adaptor cache (e.g., when protocol configs are updated)
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Remove a specific adaptor from the cache
    pub fn invalidate(&self, channel_type: i32, api_version: &str) {
        let key = AdaptorCacheKey {
            channel_type,
            api_version: api_version.to_string(),
        };
        self.cache.remove(&key);
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

    #[test]
    fn test_adaptor_cache_key() {
        let key1 = AdaptorCacheKey {
            channel_type: 1,
            api_version: "default".to_string(),
        };
        let key2 = AdaptorCacheKey {
            channel_type: 1,
            api_version: "default".to_string(),
        };
        let key3 = AdaptorCacheKey {
            channel_type: 1,
            api_version: "2024-02-01".to_string(),
        };

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
}
