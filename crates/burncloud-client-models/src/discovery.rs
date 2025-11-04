// 模型发现和搜索API模块

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::validation::ChecksumType;

/// 模型发现客户端
pub struct ModelDiscoveryClient {
    base_url: String,
    timeout: std::time::Duration,
    client: reqwest::Client,
}

/// 模型搜索请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSearchRequest {
    pub query: Option<String>,
    pub model_type: Option<ModelType>,
    pub provider: Option<String>,
    pub min_size_gb: Option<f64>,
    pub max_size_gb: Option<f64>,
    pub tags: Option<Vec<String>>,
    pub capabilities: Option<Vec<String>>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub sort_by: Option<SortBy>,
    pub sort_order: Option<SortOrder>,
}

/// 模型搜索响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSearchResponse {
    pub models: Vec<DiscoveredModel>,
    pub total_count: u64,
    pub page: u32,
    pub page_size: u32,
    pub has_next: bool,
    pub search_time_ms: u64,
}

/// 发现的模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredModel {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
    pub size_gb: f64,
    pub model_type: ModelType,
    pub provider: String,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
    pub requirements: ModelRequirements,
    pub download_url: String,
    pub checksum: String,
    pub checksum_type: ChecksumType,
    pub license: String,
    pub rating: f32,
    pub download_count: u64,
    pub last_updated: DateTime<Utc>,
    pub is_featured: bool,
    pub is_verified: bool,
    pub repository_url: Option<String>,
    pub documentation_url: Option<String>,
}

/// 模型类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModelType {
    TextGeneration,
    ChatCompletion,
    Embedding,
    CodeGeneration,
    ImageGeneration,
    Multimodal,
}

/// 模型系统要求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRequirements {
    pub min_ram_gb: f64,
    pub min_vram_gb: Option<f64>,
    pub gpu_required: bool,
    pub cpu_cores: u32,
    pub disk_space_gb: f64,
    pub supported_platforms: Vec<String>,
    pub cuda_version: Option<String>,
    pub python_version: Option<String>,
}

/// 排序方式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortBy {
    Name,
    Size,
    Rating,
    DownloadCount,
    LastUpdated,
    Relevance,
}

/// 排序顺序
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// 模型发现错误
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("网络请求失败: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("JSON解析失败: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("API错误: {status} - {message}")]
    ApiError { status: u16, message: String },
    #[error("超时错误")]
    TimeoutError,
    #[error("配置错误: {0}")]
    ConfigError(String),
}

impl ModelDiscoveryClient {
    /// 创建新的模型发现客户端
    pub fn new(base_url: String) -> Result<Self, DiscoveryError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            base_url,
            timeout: std::time::Duration::from_secs(30),
            client,
        })
    }

    /// 设置请求超时时间
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// 搜索模型
    pub async fn search_models(&self, request: ModelSearchRequest) -> Result<ModelSearchResponse, DiscoveryError> {
        let url = format!("{}/api/v1/models/search", self.base_url);

        let response = self.client
            .post(&url)
            .timeout(self.timeout)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DiscoveryError::ApiError { status, message });
        }

        let search_response: ModelSearchResponse = response.json().await?;
        Ok(search_response)
    }

    /// 获取热门模型
    pub async fn get_featured_models(&self, limit: Option<u32>) -> Result<Vec<DiscoveredModel>, DiscoveryError> {
        let request = ModelSearchRequest {
            query: None,
            model_type: None,
            provider: None,
            min_size_gb: None,
            max_size_gb: None,
            tags: None,
            capabilities: None,
            page: Some(1),
            page_size: limit,
            sort_by: Some(SortBy::Rating),
            sort_order: Some(SortOrder::Desc),
        };

        let response = self.search_models(request).await?;
        Ok(response.models.into_iter().filter(|m| m.is_featured).collect())
    }

    /// 根据模型类型获取推荐模型
    pub async fn get_recommended_by_type(&self, model_type: ModelType, limit: Option<u32>) -> Result<Vec<DiscoveredModel>, DiscoveryError> {
        let request = ModelSearchRequest {
            query: None,
            model_type: Some(model_type),
            provider: None,
            min_size_gb: None,
            max_size_gb: None,
            tags: None,
            capabilities: None,
            page: Some(1),
            page_size: limit,
            sort_by: Some(SortBy::Rating),
            sort_order: Some(SortOrder::Desc),
        };

        let response = self.search_models(request).await?;
        Ok(response.models)
    }

    /// 获取模型详细信息
    pub async fn get_model_details(&self, model_id: Uuid) -> Result<DiscoveredModel, DiscoveryError> {
        let url = format!("{}/api/v1/models/{}", self.base_url, model_id);

        let response = self.client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DiscoveryError::ApiError { status, message });
        }

        let model: DiscoveredModel = response.json().await?;
        Ok(model)
    }

    /// 检查模型可用性
    pub async fn check_model_availability(&self, model_id: Uuid) -> Result<bool, DiscoveryError> {
        let url = format!("{}/api/v1/models/{}/availability", self.base_url, model_id);

        let response = self.client
            .head(&url)
            .timeout(self.timeout)
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    /// 获取模型分类统计
    pub async fn get_category_stats(&self) -> Result<HashMap<ModelType, u64>, DiscoveryError> {
        let url = format!("{}/api/v1/models/categories/stats", self.base_url);

        let response = self.client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DiscoveryError::ApiError { status, message });
        }

        let stats: HashMap<ModelType, u64> = response.json().await?;
        Ok(stats)
    }

    /// 获取所有可用的标签
    pub async fn get_available_tags(&self) -> Result<Vec<String>, DiscoveryError> {
        let url = format!("{}/api/v1/models/tags", self.base_url);

        let response = self.client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DiscoveryError::ApiError { status, message });
        }

        let tags: Vec<String> = response.json().await?;
        Ok(tags)
    }

    /// 获取所有支持的提供商
    pub async fn get_providers(&self) -> Result<Vec<String>, DiscoveryError> {
        let url = format!("{}/api/v1/models/providers", self.base_url);

        let response = self.client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DiscoveryError::ApiError { status, message });
        }

        let providers: Vec<String> = response.json().await?;
        Ok(providers)
    }
}

impl Default for ModelSearchRequest {
    fn default() -> Self {
        Self {
            query: None,
            model_type: None,
            provider: None,
            min_size_gb: None,
            max_size_gb: None,
            tags: None,
            capabilities: None,
            page: Some(1),
            page_size: Some(20),
            sort_by: Some(SortBy::Relevance),
            sort_order: Some(SortOrder::Desc),
        }
    }
}