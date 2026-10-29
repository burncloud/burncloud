//! # BurnCloud Supply Model
//!
//! Supply-owned model capability persistence and manifest-based resolution.
//!
//! Resolver 只接收调用方准备好的 Manifest，根据设备能力过滤 Variant 并返回结果。
//! 本 crate 不负责扫描模型文件、筛选 GGUF 文件或构建 artifact 下载地址。

mod common;
mod manifest;
mod model_capability;
mod real_model_resolver;
mod resolver;

pub use burncloud_database::DatabaseError;
pub use common::current_timestamp;
pub use manifest::{ModelManifest, Variant};
pub use model_capability::{ModelCapability, ModelCapabilityInput, ModelCapabilityModel};
pub use real_model_resolver::RealModelResolver;
pub use resolver::{
    CandidateResolutionOutcome, FakeModelResolver, LocalModelUnsupported,
    LocalModelUnsupportedReason, ModelResolutionError, ModelResolutionOutcome,
    ModelResolutionRequest, ModelResolver, ResolvedModel, ResolvedModelCandidate,
};

use burncloud_service_setting::{SettingDatabase, SettingService};
use serde::Deserialize;

/// HuggingFace 模型列表中的一项。
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HfApiModel {
    #[serde(rename = "_id")]
    pub _id: String,
    pub id: String,
    pub likes: Option<i64>,
    #[serde(rename = "trendingScore")]
    pub trending_score: Option<f64>,
    pub private: Option<bool>,
    pub downloads: Option<i64>,
    pub tags: Option<Vec<String>>,
    #[serde(rename = "pipeline_tag")]
    pub pipeline_tag: Option<String>,
    #[serde(rename = "library_name")]
    pub library_name: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    #[serde(rename = "modelId")]
    pub model_id: Option<String>,
}

/// HuggingFace discovery service.
///
/// The former database-backed CRUD surface was removed by #621 because its persistence adapter
/// contained only no-op methods and no migrated table matched its HuggingFace-shaped row type.
pub struct ModelService;

impl ModelService {
    /// 从 HuggingFace API 获取模型列表。
    ///
    /// 本方法只负责模型目录发现，不参与 Resolver 的 Manifest 解析和 Variant 过滤。
    pub async fn fetch_from_huggingface(
    ) -> std::result::Result<Vec<HfApiModel>, Box<dyn std::error::Error>> {
        let host = get_huggingface_host().await?;
        let api_url = format!("{}api/models", host);
        let response = reqwest::get(&api_url).await?;
        let models: Vec<HfApiModel> = response.json().await?;
        Ok(models)
    }
}

/// 获取带缓存的 HuggingFace Host。
pub async fn get_huggingface_host() -> std::result::Result<String, Box<dyn std::error::Error>> {
    let db = SettingDatabase::new().await?;
    if let Some(host) = SettingService::get(&db, "huggingface").await? {
        return Ok(host);
    }
    let location = burncloud_service_ip::get_location().await?;
    let host = match location.as_str() {
        "CN" => "https://hf-mirror.com/",
        _ => "https://huggingface.co/",
    };
    SettingService::set(&db, "huggingface", host).await?;
    Ok(host.to_string())
}

/// 获取模型数据存储目录配置。
pub async fn get_data_dir() -> std::result::Result<String, Box<dyn std::error::Error>> {
    let db = SettingDatabase::new().await?;
    if let Some(dir) = SettingService::get(&db, "dir_data").await? {
        return Ok(dir);
    }
    let dir = "./data";
    SettingService::set(&db, "dir_data", dir).await?;
    Ok(dir.to_string())
}
