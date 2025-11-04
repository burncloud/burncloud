// 模型下载和安装功能模块

use std::path::{Path, PathBuf};
use std::fs;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use md5;
use tokio::io::AsyncWriteExt;
use crate::validation::ChecksumType;

/// 模型下载管理器
pub struct ModelDownloadManager {
    download_dir: PathBuf,
    temp_dir: PathBuf,
    max_concurrent_downloads: usize,
    client: reqwest::Client,
}

/// 下载进度信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub model_id: Uuid,
    pub model_name: String,
    pub status: DownloadStatus,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub progress_percent: f32,
    pub download_speed_bps: u64,
    pub estimated_remaining_seconds: Option<u64>,
    pub started_at: DateTime<Utc>,
    pub error_message: Option<String>,
}

/// 下载状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Verifying,
    Installing,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

/// 安装配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationConfig {
    pub auto_verify: bool,
    pub keep_temp_files: bool,
    pub create_symlink: bool,
    pub install_dependencies: bool,
    pub enable_gpu: bool,
    pub custom_install_path: Option<PathBuf>,
}

/// 模型安装信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInstallation {
    pub model_id: Uuid,
    pub install_path: PathBuf,
    pub version: String,
    pub installed_at: DateTime<Utc>,
    pub file_size: u64,
    pub checksum: String,
    pub dependencies: Vec<String>,
    pub metadata: InstallationMetadata,
}

/// 安装元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationMetadata {
    pub config_files: Vec<PathBuf>,
    pub data_files: Vec<PathBuf>,
    pub executable_files: Vec<PathBuf>,
    pub documentation: Vec<PathBuf>,
    pub symlinks: Vec<(PathBuf, PathBuf)>,
}

/// 下载/安装错误
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("网络错误: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("I/O错误: {0}")]
    IoError(#[from] std::io::Error),
    #[error("校验失败: 期望 {expected}, 实际 {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    #[error("磁盘空间不足: 需要 {required} bytes, 可用 {available} bytes")]
    InsufficientSpace { required: u64, available: u64 },
    #[error("权限不足: {0}")]
    PermissionDenied(String),
    #[error("模型已存在: {0}")]
    ModelAlreadyExists(String),
    #[error("无效的URL: {0}")]
    InvalidUrl(String),
    #[error("安装失败: {0}")]
    InstallationFailed(String),
    #[error("配置错误: {0}")]
    ConfigError(String),
    #[error("序列化错误: {0}")]
    SerializationError(#[from] serde_json::Error),
}

impl ModelDownloadManager {
    /// 创建新的下载管理器
    pub fn new(download_dir: PathBuf) -> Result<Self, DownloadError> {
        let temp_dir = download_dir.join("temp");

        // 创建必要的目录
        fs::create_dir_all(&download_dir)?;
        fs::create_dir_all(&temp_dir)?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        Ok(Self {
            download_dir,
            temp_dir,
            max_concurrent_downloads: 3,
            client,
        })
    }

    /// 设置最大并发下载数
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent_downloads = max;
        self
    }

    /// 获取下载目录
    pub fn download_dir(&self) -> &Path {
        &self.download_dir
    }

    /// 开始下载模型
    pub async fn download_model(
        &self,
        model_id: Uuid,
        model_name: String,
        download_url: String,
        expected_checksum: String,
        checksum_type: ChecksumType,
    ) -> Result<DownloadProgress, DownloadError> {
        // 验证URL
        let url = reqwest::Url::parse(&download_url)
            .map_err(|_| DownloadError::InvalidUrl(download_url.clone()))?;

        // 检查磁盘空间
        let temp_file_path = self.temp_dir.join(format!("{}.tmp", model_id));
        self.check_disk_space(&temp_file_path, &download_url).await?;

        // 创建下载进度
        let mut progress = DownloadProgress {
            model_id,
            model_name: model_name.clone(),
            status: DownloadStatus::Downloading,
            total_bytes: 0,
            downloaded_bytes: 0,
            progress_percent: 0.0,
            download_speed_bps: 0,
            estimated_remaining_seconds: None,
            started_at: Utc::now(),
            error_message: None,
        };

        // 开始下载
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(DownloadError::InvalidUrl(
                format!("HTTP error: {}", response.status())
            ));
        }

        progress.total_bytes = response.content_length().unwrap_or(0);

        // 下载文件
        let mut file = tokio::fs::File::create(&temp_file_path).await?;
        let mut downloaded = 0u64;
        let start_time = std::time::Instant::now();

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            // 更新进度
            progress.downloaded_bytes = downloaded;
            progress.progress_percent = if progress.total_bytes > 0 {
                (downloaded as f32 / progress.total_bytes as f32) * 100.0
            } else {
                0.0
            };

            let elapsed = start_time.elapsed().as_secs();
            if elapsed > 0 {
                progress.download_speed_bps = downloaded / elapsed;
                if progress.download_speed_bps > 0 {
                    let remaining_bytes = progress.total_bytes.saturating_sub(downloaded);
                    progress.estimated_remaining_seconds = Some(remaining_bytes / progress.download_speed_bps);
                }
            }
        }

        file.flush().await?;
        drop(file);

        // 验证校验和
        progress.status = DownloadStatus::Verifying;
        self.verify_checksum(&temp_file_path, &expected_checksum, checksum_type).await?;

        // 移动文件到最终位置
        let final_path = self.download_dir.join(&model_name);
        tokio::fs::rename(&temp_file_path, &final_path).await?;

        progress.status = DownloadStatus::Completed;
        Ok(progress)
    }

    /// 安装模型
    pub async fn install_model(
        &self,
        model_id: Uuid,
        model_path: PathBuf,
        config: InstallationConfig,
    ) -> Result<ModelInstallation, DownloadError> {
        // 确定安装路径
        let install_path = config.custom_install_path
            .unwrap_or_else(|| self.download_dir.join("installed").join(model_id.to_string()));

        // 创建安装目录
        fs::create_dir_all(&install_path)?;

        // 复制或移动模型文件
        let model_file_name = model_path.file_name()
            .ok_or_else(|| DownloadError::ConfigError("无效的模型文件路径".to_string()))?;
        let target_path = install_path.join(model_file_name);

        if config.create_symlink {
            // 创建符号链接
            #[cfg(unix)]
            std::os::unix::fs::symlink(&model_path, &target_path)?;
            #[cfg(windows)]
            std::os::windows::fs::symlink_file(&model_path, &target_path)?;
        } else {
            // 复制文件
            tokio::fs::copy(&model_path, &target_path).await?;
        }

        // 获取文件大小
        let metadata = tokio::fs::metadata(&target_path).await?;
        let file_size = metadata.len();

        // 计算校验和
        let checksum = if config.auto_verify {
            self.calculate_checksum(&target_path, ChecksumType::SHA256).await?
        } else {
            String::new()
        };

        // 创建配置文件
        let config_path = install_path.join("model.json");
        let model_config = serde_json::json!({
            "model_id": model_id,
            "installed_at": Utc::now(),
            "version": "1.0.0",
            "file_size": file_size,
            "checksum": checksum
        });
        tokio::fs::write(&config_path, serde_json::to_string_pretty(&model_config)?).await?;

        // 清理临时文件
        if !config.keep_temp_files {
            if model_path.starts_with(&self.temp_dir) {
                let _ = tokio::fs::remove_file(&model_path).await;
            }
        }

        let installation = ModelInstallation {
            model_id,
            install_path: install_path.clone(),
            version: "1.0.0".to_string(),
            installed_at: Utc::now(),
            file_size,
            checksum,
            dependencies: vec![],
            metadata: InstallationMetadata {
                config_files: vec![config_path],
                data_files: vec![target_path.clone()],
                executable_files: vec![],
                documentation: vec![],
                symlinks: if config.create_symlink {
                    vec![(model_path, target_path)]
                } else {
                    vec![]
                },
            },
        };

        Ok(installation)
    }

    /// 暂停下载
    pub async fn pause_download(&self, _model_id: Uuid) -> Result<(), DownloadError> {
        // 实现下载暂停逻辑
        // 这里需要与下载任务管理器配合
        Ok(())
    }

    /// 恢复下载
    pub async fn resume_download(&self, _model_id: Uuid) -> Result<(), DownloadError> {
        // 实现下载恢复逻辑
        // 支持断点续传
        Ok(())
    }

    /// 取消下载
    pub async fn cancel_download(&self, model_id: Uuid) -> Result<(), DownloadError> {
        // 清理临时文件
        let temp_file_path = self.temp_dir.join(format!("{}.tmp", model_id));
        if temp_file_path.exists() {
            tokio::fs::remove_file(&temp_file_path).await?;
        }
        Ok(())
    }

    /// 卸载模型
    pub async fn uninstall_model(&self, model_id: Uuid) -> Result<(), DownloadError> {
        let install_path = self.download_dir.join("installed").join(model_id.to_string());
        if install_path.exists() {
            tokio::fs::remove_dir_all(&install_path).await?;
        }
        Ok(())
    }

    /// 获取已安装的模型列表
    pub async fn get_installed_models(&self) -> Result<Vec<ModelInstallation>, DownloadError> {
        let installed_dir = self.download_dir.join("installed");
        if !installed_dir.exists() {
            return Ok(vec![]);
        }

        let mut installations = vec![];
        let mut entries = tokio::fs::read_dir(&installed_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let config_path = entry.path().join("model.json");
                if config_path.exists() {
                    if let Ok(config_content) = tokio::fs::read_to_string(&config_path).await {
                        if let Ok(installation) = serde_json::from_str::<ModelInstallation>(&config_content) {
                            installations.push(installation);
                        }
                    }
                }
            }
        }

        Ok(installations)
    }

    /// 检查磁盘空间
    async fn check_disk_space(&self, file_path: &Path, download_url: &str) -> Result<(), DownloadError> {
        // 获取文件大小（通过HEAD请求）
        let response = self.client.head(download_url).send().await?;
        let required_size = response.content_length().unwrap_or(0);

        // 检查可用磁盘空间
        let available_space = self.get_available_disk_space(file_path)?;

        if required_size > available_space {
            return Err(DownloadError::InsufficientSpace {
                required: required_size,
                available: available_space,
            });
        }

        Ok(())
    }

    /// 获取可用磁盘空间
    fn get_available_disk_space(&self, _path: &Path) -> Result<u64, DownloadError> {
        // 简化实现，实际应该使用系统API
        // 这里返回一个大致的可用空间
        Ok(10_000_000_000) // 10GB
    }

    /// 验证校验和
    async fn verify_checksum(
        &self,
        file_path: &Path,
        expected: &str,
        checksum_type: ChecksumType,
    ) -> Result<(), DownloadError> {
        let actual = self.calculate_checksum(file_path, checksum_type).await?;

        if actual.to_lowercase() != expected.to_lowercase() {
            return Err(DownloadError::ChecksumMismatch {
                expected: expected.to_string(),
                actual,
            });
        }

        Ok(())
    }

    /// 计算文件校验和
    async fn calculate_checksum(
        &self,
        file_path: &Path,
        checksum_type: ChecksumType,
    ) -> Result<String, DownloadError> {
        let content = tokio::fs::read(file_path).await?;

        let hash = match checksum_type {
            ChecksumType::MD5 => {
                let digest = md5::compute(&content);
                format!("{:x}", digest)
            }
            ChecksumType::SHA256 => {
                let mut hasher = Sha256::new();
                hasher.update(&content);
                format!("{:x}", hasher.finalize())
            }
            ChecksumType::SHA512 => {
                use sha2::Sha512;
                let mut hasher = Sha512::new();
                hasher.update(&content);
                format!("{:x}", hasher.finalize())
            }
        };

        Ok(hash)
    }
}

impl Default for InstallationConfig {
    fn default() -> Self {
        Self {
            auto_verify: true,
            keep_temp_files: false,
            create_symlink: false,
            install_dependencies: true,
            enable_gpu: false,
            custom_install_path: None,
        }
    }
}

// 添加必要的use语句
use futures_util::stream::StreamExt;