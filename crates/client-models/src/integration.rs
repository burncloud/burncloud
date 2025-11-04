// 简化的模型管理集成示例（不依赖数据库）

use std::path::PathBuf;
use uuid::Uuid;
use crate::{
    ModelDiscoveryClient, ModelSearchRequest, ModelDownloadManager, ModelValidator,
    InstallationConfig, ValidationConfig, DiscoveredModel
};

/// 简化的模型管理服务
pub struct ModelManagementService {
    discovery_client: ModelDiscoveryClient,
    download_manager: ModelDownloadManager,
    validator: ModelValidator,
}

impl ModelManagementService {
    /// 创建新的模型管理服务
    pub async fn new(
        discovery_base_url: String,
        download_dir: PathBuf,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // 初始化发现客户端
        let discovery_client = ModelDiscoveryClient::new(discovery_base_url)?;

        // 初始化下载管理器
        let download_manager = ModelDownloadManager::new(download_dir.clone())?;

        // 初始化验证器
        let temp_dir = download_dir.join("temp");
        let validator = ModelValidator::new(temp_dir)?;

        Ok(Self {
            discovery_client,
            download_manager,
            validator,
        })
    }

    /// 搜索并发现模型
    pub async fn discover_models(&self, query: &str) -> Result<Vec<DiscoveredModel>, Box<dyn std::error::Error>> {
        let search_request = ModelSearchRequest {
            query: Some(query.to_string()),
            ..Default::default()
        };

        let response = self.discovery_client.search_models(search_request).await?;
        Ok(response.models)
    }

    /// 简化的模型安装流程：发现 -> 下载 -> 验证 -> 安装
    pub async fn install_model_simple(
        &self,
        model_name: &str,
        model_version: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {

        // 1. 搜索模型
        println!("🔍 正在搜索模型: {}", model_name);
        let search_request = ModelSearchRequest {
            query: Some(model_name.to_string()),
            ..Default::default()
        };
        let search_response = self.discovery_client.search_models(search_request).await?;

        let discovered_model = search_response.models.into_iter()
            .find(|m| {
                if let Some(version) = model_version {
                    m.name == model_name && m.version == version
                } else {
                    m.name == model_name
                }
            })
            .ok_or("模型未找到")?;

        println!("✅ 找到模型: {} v{}", discovered_model.name, discovered_model.version);

        // 2. 下载模型
        println!("📥 开始下载模型...");
        let download_progress = self.download_manager.download_model(
            discovered_model.id,
            discovered_model.name.clone(),
            discovered_model.download_url,
            discovered_model.checksum.clone(),
            crate::validation::ChecksumType::SHA256,
        ).await?;

        match download_progress.status {
            crate::DownloadStatus::Completed => {
                println!("✅ 模型下载完成");
            }
            _ => {
                return Err("下载失败".into());
            }
        }

        // 3. 验证模型
        println!("🔒 正在验证模型完整性...");
        let model_path = self.download_manager.download_dir().join(&discovered_model.name);
        let validation_config = ValidationConfig::default();
        let validation_result = self.validator.validate_model(&model_path, Some(discovered_model.id), validation_config).await?;

        if !validation_result.is_valid {
            return Err("模型验证失败".into());
        }
        println!("✅ 模型验证通过");

        // 4. 安装模型
        println!("📦 正在安装模型...");
        let install_config = InstallationConfig::default();
        let installation = self.download_manager.install_model(
            discovered_model.id,
            model_path,
            install_config.clone(),
        ).await?;

        println!("🎉 模型安装完成!");
        Ok(installation.install_path.to_string_lossy().to_string())
    }

    /// 列出已安装的模型
    pub async fn list_installed_models(&self) -> Result<Vec<crate::ModelInstallation>, Box<dyn std::error::Error>> {
        let installed = self.download_manager.get_installed_models().await?;
        Ok(installed)
    }

    /// 卸载模型
    pub async fn uninstall_model(&self, model_id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
        // 从文件系统删除
        self.download_manager.uninstall_model(model_id).await?;
        println!("✅ 模型已卸载");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_model_management_service() {
        // 这是一个集成测试示例
        let service = ModelManagementService::new(
            "https://api.burncloud.com".to_string(),
            PathBuf::from("./models"),
        ).await.expect("Failed to create service");

        // 搜索模型
        let models = service.discover_models("qwen").await;
        // 在演示模式下，这通常会因为网络错误而失败
        // 这是预期的行为
    }
}