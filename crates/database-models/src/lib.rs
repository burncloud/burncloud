//! # BurnCloud Database Models
//!
//! AI 模型信息数据库管理库，基于 `burncloud-database` 构建

use burncloud_database::{Database, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// AI 模型信息结构体
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ModelInfo {
    /// 模型唯一标识符
    pub model_id: String,
    /// 是否私有模型 (0=false, 1=true)
    #[serde(deserialize_with = "bool_from_int", serialize_with = "bool_to_int")]
    pub private: bool,

    // 模型分类
    /// 模型管道类型
    pub pipeline_tag: Option<String>,
    /// 库名称
    pub library_name: Option<String>,
    /// 模型类型
    pub model_type: Option<String>,

    // 统计信息
    /// 下载次数
    pub downloads: i64,
    /// 点赞数
    pub likes: i64,

    // 版本信息
    /// Git SHA
    pub sha: Option<String>,
    /// 最后修改时间
    pub last_modified: Option<String>,
    /// 是否需要授权访问 (0=false, 1=true)
    #[serde(deserialize_with = "bool_from_int", serialize_with = "bool_to_int")]
    pub gated: bool,
    /// 是否已禁用 (0=false, 1=true)
    #[serde(deserialize_with = "bool_from_int", serialize_with = "bool_to_int")]
    pub disabled: bool,

    // JSON 存储字段
    /// 标签列表 (JSON数组)
    pub tags: String,
    /// 模型配置 (JSON对象)
    pub config: String,
    /// 示例数据 (JSON数组)
    pub widget_data: String,
    /// 卡片元数据 (JSON对象)
    pub card_data: String,
    /// Transformers信息 (JSON对象)
    pub transformers_info: String,
    /// 相关文件列表 (JSON数组)
    pub siblings: String,
    /// 关联空间列表 (JSON数组)
    pub spaces: String,
    /// SafeTensors信息 (JSON对象)
    pub safetensors: String,

    // 存储信息
    /// 使用的存储空间(字节)
    pub used_storage: i64,
    /// 文件名
    pub filename: Option<String>,
    /// 文件大小(字节)
    pub size: i64,

    // 时间戳
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 辅助函数：从整数反序列化布尔值
fn bool_from_int<'de, D>(deserializer: D) -> std::result::Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match i64::deserialize(deserializer)? {
        0 => Ok(false),
        _ => Ok(true),
    }
}

/// 辅助函数：将布尔值序列化为整数
fn bool_to_int<S>(value: &bool, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_i64(if *value { 1 } else { 0 })
}

/// 模型数据库管理器
pub struct ModelDatabase {
    db: Database,
}

impl ModelDatabase {
    /// 创建新的模型数据库实例
    pub async fn new() -> Result<Self> {
        let db = Database::new().await?;
        let model_db = Self { db };
        model_db.init_tables().await?;
        Ok(model_db)
    }

    /// 初始化数据库表结构
    async fn init_tables(&self) -> Result<()> {
        // 创建 models 表
        let create_table_sql = r#"
            CREATE TABLE IF NOT EXISTS models (
                -- 基础字段
                model_id TEXT PRIMARY KEY,
                private INTEGER NOT NULL DEFAULT 0,

                -- 模型分类
                pipeline_tag TEXT,
                library_name TEXT,
                model_type TEXT,

                -- 统计信息
                downloads INTEGER DEFAULT 0,
                likes INTEGER DEFAULT 0,

                -- 版本信息
                sha TEXT,
                last_modified DATETIME,
                gated INTEGER DEFAULT 0,
                disabled INTEGER DEFAULT 0,

                -- JSON 存储字段
                tags TEXT DEFAULT '[]',
                config TEXT DEFAULT '{}',
                widget_data TEXT DEFAULT '[]',
                card_data TEXT DEFAULT '{}',
                transformers_info TEXT DEFAULT '{}',
                siblings TEXT DEFAULT '[]',
                spaces TEXT DEFAULT '[]',
                safetensors TEXT DEFAULT '{}',

                -- 存储信息
                used_storage INTEGER DEFAULT 0,
                filename TEXT,
                size INTEGER DEFAULT 0,

                -- 时间戳
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#;

        self.db.execute_query(create_table_sql).await?;

        // 添加新字段 (如果不存在)
        let migrations = [
            "ALTER TABLE models ADD COLUMN filename TEXT",
            "ALTER TABLE models ADD COLUMN size INTEGER DEFAULT 0",
        ];

        for migration_sql in &migrations {
            // 忽略已存在列的错误
            let _ = self.db.execute_query(migration_sql).await;
        }

        // 创建索引
        let indexes = [
            "CREATE INDEX IF NOT EXISTS idx_models_pipeline_tag ON models(pipeline_tag)",
            "CREATE INDEX IF NOT EXISTS idx_models_library_name ON models(library_name)",
            "CREATE INDEX IF NOT EXISTS idx_models_downloads ON models(downloads DESC)",
            "CREATE INDEX IF NOT EXISTS idx_models_likes ON models(likes DESC)",
            "CREATE INDEX IF NOT EXISTS idx_models_created_at ON models(created_at)",
            "CREATE INDEX IF NOT EXISTS idx_models_private ON models(private)",
        ];

        for index_sql in &indexes {
            self.db.execute_query(index_sql).await?;
        }

        Ok(())
    }

    /// 添加模型信息
    pub async fn add_model(&self, model: &ModelInfo) -> Result<()> {
        let sql = r#"
            INSERT OR REPLACE INTO models (
                model_id, private, pipeline_tag, library_name, model_type,
                downloads, likes, sha, last_modified, gated, disabled,
                tags, config, widget_data, card_data, transformers_info,
                siblings, spaces, safetensors, used_storage, filename, size,
                created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22,
                COALESCE((SELECT created_at FROM models WHERE model_id = ?1), CURRENT_TIMESTAMP),
                CURRENT_TIMESTAMP
            )
        "#;

        let params = vec![
            model.model_id.clone(),
            if model.private { "1" } else { "0" }.to_string(),
            model.pipeline_tag.clone().unwrap_or_default(),
            model.library_name.clone().unwrap_or_default(),
            model.model_type.clone().unwrap_or_default(),
            model.downloads.to_string(),
            model.likes.to_string(),
            model.sha.clone().unwrap_or_default(),
            model.last_modified.clone().unwrap_or_default(),
            if model.gated { "1" } else { "0" }.to_string(),
            if model.disabled { "1" } else { "0" }.to_string(),
            model.tags.clone(),
            model.config.clone(),
            model.widget_data.clone(),
            model.card_data.clone(),
            model.transformers_info.clone(),
            model.siblings.clone(),
            model.spaces.clone(),
            model.safetensors.clone(),
            model.used_storage.to_string(),
            model.filename.clone().unwrap_or_default(),
            model.size.to_string(),
        ];

        self.db.execute_query_with_params(sql, params).await?;
        Ok(())
    }

    /// 删除模型
    pub async fn delete(&self, model_id: &str) -> Result<()> {
        let sql = "DELETE FROM models WHERE model_id = ?1";
        let params = vec![model_id.to_string()];
        self.db.execute_query_with_params(sql, params).await?;
        Ok(())
    }

    /// 根据模型ID获取模型信息
    pub async fn get_model(&self, model_id: &str) -> Result<Option<ModelInfo>> {
        let sql = "SELECT * FROM models WHERE model_id = ?1";
        let params = vec![model_id.to_string()];
        let rows = self.db.query_with_params(sql, params).await?;

        if rows.is_empty() {
            Ok(None)
        } else {
            let model: ModelInfo = sqlx::FromRow::from_row(&rows[0])?;
            Ok(Some(model))
        }
    }

    /// 获取所有模型列表
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let sql = "SELECT * FROM models ORDER BY created_at DESC";
        self.db.fetch_all::<ModelInfo>(sql).await
    }

    /// 根据管道类型搜索模型
    pub async fn search_by_pipeline(&self, pipeline_tag: &str) -> Result<Vec<ModelInfo>> {
        let sql = "SELECT * FROM models WHERE pipeline_tag = ?1 ORDER BY downloads DESC";
        let params = vec![pipeline_tag.to_string()];
        let rows = self.db.query_with_params(sql, params).await?;

        let mut models = Vec::new();
        for row in rows {
            let model: ModelInfo = sqlx::FromRow::from_row(&row)?;
            models.push(model);
        }
        Ok(models)
    }

    /// 获取热门模型（按下载量排序）
    pub async fn get_popular_models(&self, limit: i64) -> Result<Vec<ModelInfo>> {
        let sql = "SELECT * FROM models ORDER BY downloads DESC LIMIT ?1";
        let params = vec![limit.to_string()];
        let rows = self.db.query_with_params(sql, params).await?;

        let mut models = Vec::new();
        for row in rows {
            let model: ModelInfo = sqlx::FromRow::from_row(&row)?;
            models.push(model);
        }
        Ok(models)
    }

    /// 关闭数据库连接
    pub async fn close(self) -> Result<()> {
        self.db.close().await
    }
}

/// 重新导出 burncloud_database 的公共类型
pub use burncloud_database::{DatabaseConnection, DatabaseError};