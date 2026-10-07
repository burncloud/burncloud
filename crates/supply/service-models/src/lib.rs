//! # BurnCloud Service Models
//!
//! 模型服务层，提供简洁的增删改查接口

mod manifest;
mod resolver;

pub use manifest::{ModelManifest, Variant};
pub use resolver::{
    FakeModelResolver, LocalModelUnsupported, LocalModelUnsupportedReason, ModelResolutionError,
    ModelResolutionOutcome, ModelResolutionRequest, ModelResolver, ResolvedModel,
};

use burncloud_service_setting::{SettingDatabase, SettingService};
use serde::Deserialize;

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

#[derive(Debug, Clone, Deserialize)]
pub struct HfFileItem {
    #[serde(rename = "type")]
    pub file_type: String,
    pub oid: String,
    pub size: i64,
    pub path: String,
}

/// HuggingFace discovery service.
///
/// The former database-backed CRUD surface was removed by #621 because its persistence adapter
/// contained only no-op methods and no migrated table matched its HuggingFace-shaped row type.
pub struct ModelService;

impl ModelService {
    /// 从 HuggingFace API 获取模型列表
    pub async fn fetch_from_huggingface(
    ) -> std::result::Result<Vec<HfApiModel>, Box<dyn std::error::Error>> {
        let host = get_huggingface_host().await?;
        let api_url = format!("{}api/models", host);
        let response = reqwest::get(&api_url).await?;
        let models: Vec<HfApiModel> = response.json().await?;
        Ok(models)
    }
}

/// 获取 HuggingFace Host（带缓存）
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

/// 获取模型的所有文件列表（递归遍历）
pub async fn get_model_files(
    model_id: &str,
) -> std::result::Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    let host = get_huggingface_host().await?;
    let mut result = Vec::new();
    fetch_files_recursive(&host, model_id, "main", &mut result).await?;
    Ok(result)
}

#[allow(
    clippy::type_complexity,
    reason = "recursive async traversal requires a boxed Future whose borrowed inputs share the same lifetime"
)]
fn fetch_files_recursive<'a>(
    host: &'a str,
    model_id: &'a str,
    path: &'a str,
    result: &'a mut Vec<Vec<String>>,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = std::result::Result<(), Box<dyn std::error::Error>>> + 'a>,
> {
    Box::pin(async move {
        let url = format!("{}api/models/{}/tree/{}", host, model_id, path);
        let response = reqwest::get(&url).await?;
        let items: Vec<HfFileItem> = response.json().await?;
        for item in items {
            if item.file_type == "file" {
                result.push(vec![
                    item.file_type.clone(),
                    item.oid.clone(),
                    item.size.to_string(),
                    item.path.clone(),
                ]);
            } else if item.file_type == "directory" {
                let sub_path = format!("{}/{}", path, item.path);
                fetch_files_recursive(host, model_id, &sub_path, result).await?;
            }
        }
        Ok(())
    })
}

/// 从文件列表中筛选出所有 GGUF 文件
pub fn filter_gguf_files(files: &[Vec<String>]) -> Vec<Vec<String>> {
    files
        .iter()
        .filter(|f| f[3].to_lowercase().ends_with(".gguf"))
        .cloned()
        .collect()
}

/// 获取数据存储目录
pub async fn get_data_dir() -> std::result::Result<String, Box<dyn std::error::Error>> {
    let db = SettingDatabase::new().await?;
    if let Some(dir) = SettingService::get(&db, "dir_data").await? {
        return Ok(dir);
    }
    let dir = "./data";
    SettingService::set(&db, "dir_data", dir).await?;
    Ok(dir.to_string())
}

/// 构建下载 URL
pub async fn build_download_url(
    model_id: &str,
    path: &str,
) -> std::result::Result<String, Box<dyn std::error::Error>> {
    let host = get_huggingface_host().await?;
    Ok(format!(
        "{}{}/resolve/main/{}?download=true",
        host, model_id, path
    ))
}
