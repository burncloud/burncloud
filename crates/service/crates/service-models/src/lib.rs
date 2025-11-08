//! # BurnCloud Service Models
//!
//! 模型服务层，提供简洁的增删改查接口

use burncloud_database_models::ModelDatabase;
use serde::Deserialize;

type Result<T> = std::result::Result<T, burncloud_database_models::DatabaseError>;

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
    pub async fn create(&self, model: &burncloud_database_models::ModelInfo) -> Result<()> {
        self.db.add_model(model).await
    }

    /// 删除模型
    pub async fn delete(&self, model_id: &str) -> Result<()> {
        self.db.delete(model_id).await
    }

    /// 更新模型（使用 add_model 的 INSERT OR REPLACE 逻辑）
    pub async fn update(&self, model: &burncloud_database_models::ModelInfo) -> Result<()> {
        self.db.add_model(model).await
    }

    /// 根据ID查询模型
    pub async fn get(&self, model_id: &str) -> Result<Option<burncloud_database_models::ModelInfo>> {
        self.db.get_model(model_id).await
    }

    /// 查询所有模型
    pub async fn list(&self) -> Result<Vec<burncloud_database_models::ModelInfo>> {
        self.db.list_models().await
    }

    /// 根据管道类型搜索
    pub async fn search_by_pipeline(&self, pipeline_tag: &str) -> Result<Vec<burncloud_database_models::ModelInfo>> {
        self.db.search_by_pipeline(pipeline_tag).await
    }

    /// 获取热门模型
    pub async fn get_popular(&self, limit: i64) -> Result<Vec<burncloud_database_models::ModelInfo>> {
        self.db.get_popular_models(limit).await
    }

    /// 关闭服务
    pub async fn close(self) -> Result<()> {
        self.db.close().await
    }

    /// 从 HuggingFace API 获取模型列表
    pub async fn fetch_from_huggingface() -> std::result::Result<Vec<HfApiModel>, Box<dyn std::error::Error>> {
        // 获取地区
        let location = burncloud_service_ip::get_location().await?;

        // 根据地区选择 API 端点
        let api_url = match location.as_str() {
            "CN" => "https://hf-mirror.com/api/models",
            _ => "https://huggingface.co/api/models",
        };

        // 请求数据
        let response = reqwest::get(api_url).await?;
        let models: Vec<HfApiModel> = response.json().await?;

        Ok(models)
    }
}

/// 重新导出常用类型
pub use burncloud_database_models::{DatabaseError, ModelInfo};
