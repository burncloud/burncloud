//! # BurnCloud Service Models
//!
//! 模型服务层，提供简洁的增删改查接口

use burncloud_database_models::ModelDatabase;

type Result<T> = std::result::Result<T, burncloud_database_models::DatabaseError>;

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
}

/// 重新导出常用类型
pub use burncloud_database_models::{DatabaseError, ModelInfo};
