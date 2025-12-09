//! # BurnCloud Service Models
//!
//! 模型服务层，提供简洁的增删改查接口

use burncloud_database_models::ModelDatabase;
use burncloud_service_setting::SettingService;
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
    pub async fn create(&self, model: &burncloud_database_models::ModelInfo) -> Result<()> {
        self.db.add_model(model).await
    }

    /// 删除模型
    pub async fn delete(&self, model_id: &str) -> Result<()> {
        // 尝试清理物理文件
        if let Ok(Some(model)) = self.get(model_id).await {
            // 忽略文件清理错误，确保数据库记录被删除
            let _ = self
                .cleanup_files(model_id, model.filename.as_deref())
                .await;
        }
        self.db.delete(model_id).await
    }

    /// 清理文件辅助函数
    async fn cleanup_files(
        &self,
        model_id: &str,
        filename: Option<&str>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let base_dir = get_data_dir().await?;
        let model_dir = std::path::Path::new(&base_dir).join(model_id);

        if !model_dir.exists() {
            return Ok(());
        }

        if let Some(fname) = filename {
            if fname.trim().is_empty() {
                return Ok(());
            }

            // 去掉 .gguf 后缀
            let prefix = if fname.to_lowercase().ends_with(".gguf") {
                &fname[..fname.len() - 5]
            } else {
                fname
            };

            if prefix.is_empty() {
                return Ok(());
            }

            let mut entries = tokio::fs::read_dir(&model_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with(prefix) {
                            tokio::fs::remove_file(&path).await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// 更新模型（使用 add_model 的 INSERT OR REPLACE 逻辑）
    pub async fn update(&self, model: &burncloud_database_models::ModelInfo) -> Result<()> {
        self.db.add_model(model).await
    }

    /// 根据ID查询模型
    pub async fn get(
        &self,
        model_id: &str,
    ) -> Result<Option<burncloud_database_models::ModelInfo>> {
        self.db.get_model(model_id).await
    }

    /// 查询所有模型
    pub async fn list(&self) -> Result<Vec<burncloud_database_models::ModelInfo>> {
        self.db.list_models().await
    }

    /// 根据管道类型搜索
    pub async fn search_by_pipeline(
        &self,
        pipeline_tag: &str,
    ) -> Result<Vec<burncloud_database_models::ModelInfo>> {
        self.db.search_by_pipeline(pipeline_tag).await
    }

    /// 获取热门模型
    pub async fn get_popular(
        &self,
        limit: i64,
    ) -> Result<Vec<burncloud_database_models::ModelInfo>> {
        self.db.get_popular_models(limit).await
    }

    /// 关闭服务
    pub async fn close(self) -> Result<()> {
        self.db.close().await
    }

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
    let setting = SettingService::new().await?;

    // 先查询缓存
    if let Some(host) = setting.get("huggingface").await? {
        return Ok(host);
    }

    // 没有缓存，根据地区设置
    let location = burncloud_service_ip::get_location().await?;
    let host = match location.as_str() {
        "CN" => "https://hf-mirror.com/",
        _ => "https://huggingface.co/",
    };

    // 保存到数据库
    setting.set("huggingface", host).await?;

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
    let setting = SettingService::new().await?;

    if let Some(dir) = setting.get("dir_data").await? {
        return Ok(dir);
    }

    let dir = "./data";
    setting.set("dir_data", dir).await?;
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

/// 下载模型文件
pub async fn download_model_file(
    model_id: &str,
    path: &str,
) -> std::result::Result<String, Box<dyn std::error::Error>> {
    let url = build_download_url(model_id, path).await?;
    let base_dir = get_data_dir().await?;
    let download_dir = format!("{}/{}", base_dir, model_id);

    let manager = burncloud_download::DownloadManager::new().await?;
    let gid = manager.add_download(&url, Some(&download_dir)).await?;

    Ok(gid)
}

/// 重新导出常用类型
pub use burncloud_database_models::{DatabaseError, ModelInfo};
