//! # BurnCloud Service Models
//!
//! 模型服务层，提供简洁的增删改查接口

mod resolver;

pub use resolver::{
    FakeModelResolver, LocalModelUnsupported, LocalModelUnsupportedReason, ModelResolutionError,
    ModelResolutionOutcome, ModelResolutionRequest, ModelResolver, ResolvedModel,
};

use burncloud_database_model::ModelDatabase;
use burncloud_service_setting::{SettingDatabase, SettingService};
use serde::Deserialize;

type Result<T> = std::result::Result<T, burncloud_database_model::DatabaseError>;

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

/// 模型服务
pub struct ModelService {
    db: ModelDatabase,
}

impl ModelService {
    /// 创建新的模型服务实例
    pub async fn new() -> Result<Self> {
        Ok(Self {
            db: ModelDatabase::new().await?,
        })
    }

    /// 添加模型
    pub async fn create(&self, model: &burncloud_database_model::ModelInfo) -> Result<()> {
        self.db.add_model(model).await
    }

    /// 删除模型
    pub async fn delete(&self, model_id: &str) -> Result<()> {
        if let Ok(Some(model)) = self.get(model_id).await {
            let _ = self.cleanup_files(model_id, model.filename.as_deref()).await;
        }
        self.db.delete(model_id).await
    }

    async fn cleanup_files(&self, model_id: &str, filename: Option<&str>) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let base_dir = get_data_dir().await?;
        let model_dir = std::path::Path::new(&base_dir).join(model_id);
        if !model_dir.exists() { return Ok(()); }
        if let Some(fname) = filename {
            if fname.trim().is_empty() { return Ok(()); }
            let prefix = if fname.to_lowercase().ends_with(".gguf") { &fname[..fname.len() - 5] } else { fname };
            if prefix.is_empty() { return Ok(()); }
            let mut entries = tokio::fs::read_dir(&model_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with(prefix) { tokio::fs::remove_file(&path).await?; }
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn update(&self, model: &burncloud_database_model::ModelInfo) -> Result<()> { self.db.add_model(model).await }
    pub async fn get(&self, model_id: &str) -> Result<Option<burncloud_database_model::ModelInfo>> { self.db.get_model(model_id).await }
    pub async fn list(&self) -> Result<Vec<burncloud_database_model::ModelInfo>> { self.db.list_models().await }
    pub async fn search_by_pipeline(&self, pipeline_tag: &str) -> Result<Vec<burncloud_database_model::ModelInfo>> { self.db.search_by_pipeline(pipeline_tag).await }
    pub async fn get_popular(&self, limit: i64) -> Result<Vec<burncloud_database_model::ModelInfo>> { self.db.get_popular_models(limit).await }
    pub async fn close(self) -> Result<()> { self.db.close().await }

    pub async fn fetch_from_huggingface() -> std::result::Result<Vec<HfApiModel>, Box<dyn std::error::Error>> {
        let host = get_huggingface_host().await?;
        let response = reqwest::get(format!("{}api/models", host)).await?;
        Ok(response.json().await?)
    }
}

pub async fn get_huggingface_host() -> std::result::Result<String, Box<dyn std::error::Error>> {
    let db = SettingDatabase::new().await?;
    if let Some(host) = SettingService::get(&db, "huggingface").await? { return Ok(host); }
    let location = burncloud_service_ip::get_location().await?;
    let host = match location.as_str() { "CN" => "https://hf-mirror.com/", _ => "https://huggingface.co/" };
    SettingService::set(&db, "huggingface", host).await?;
    Ok(host.to_string())
}

pub async fn get_model_files(model_id: &str) -> std::result::Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    let host = get_huggingface_host().await?;
    let mut result = Vec::new();
    fetch_files_recursive(&host, model_id, "main", &mut result).await?;
    Ok(result)
}

#[allow(clippy::type_complexity)]
fn fetch_files_recursive<'a>(host: &'a str, model_id: &'a str, path: &'a str, result: &'a mut Vec<Vec<String>>) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<(), Box<dyn std::error::Error>>> + 'a>> {
    Box::pin(async move {
        let response = reqwest::get(format!("{}api/models/{}/tree/{}", host, model_id, path)).await?;
        let items: Vec<HfFileItem> = response.json().await?;
        for item in items {
            if item.file_type == "file" {
                result.push(vec![item.file_type.clone(), item.oid.clone(), item.size.to_string(), item.path.clone()]);
            } else if item.file_type == "directory" {
                fetch_files_recursive(host, model_id, &format!("{}/{}", path, item.path), result).await?;
            }
        }
        Ok(())
    })
}

pub fn filter_gguf_files(files: &[Vec<String>]) -> Vec<Vec<String>> {
    files.iter().filter(|f| f[3].to_lowercase().ends_with(".gguf")).cloned().collect()
}

pub async fn get_data_dir() -> std::result::Result<String, Box<dyn std::error::Error>> {
    let db = SettingDatabase::new().await?;
    if let Some(dir) = SettingService::get(&db, "dir_data").await? { return Ok(dir); }
    let dir = "./data";
    SettingService::set(&db, "dir_data", dir).await?;
    Ok(dir.to_string())
}

pub async fn build_download_url(model_id: &str, path: &str) -> std::result::Result<String, Box<dyn std::error::Error>> {
    let host = get_huggingface_host().await?;
    Ok(format!("{}{}/resolve/main/{}?download=true", host, model_id, path))
}
