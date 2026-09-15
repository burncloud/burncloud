//! # BurnCloud Service Inference
//!
//! 本地推理服务管理模块，负责 `llama-server` 等推理后端的进程管理。

mod error;

pub use error::{InferenceError, Result};

use burncloud_common::types::Channel;
use burncloud_database::Database;
use burncloud_database_channel::{ChannelAbilityInput, ChannelAbilityModel, ChannelProviderModel};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// 推理实例状态
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub enum InstanceStatus {
    Stopped,
    Starting,
    Running,
    Failed(String),
}

/// 推理实例配置
#[derive(Debug, Clone)]
pub struct InferenceConfig {
    pub model_id: String,
    pub file_path: String, // GGUF 文件的绝对路径
    pub port: u16,
    pub context_size: u32,
    pub gpu_layers: i32, // -1 for all
}

/// 推理服务管理器
pub struct InferenceService {
    // 存储活跃的进程句柄: Map<ModelID, ChildProcess>
    processes: Arc<Mutex<HashMap<String, Child>>>,
    // 存储实例状态: Map<ModelID, Status>
    statuses: Arc<Mutex<HashMap<String, InstanceStatus>>>,
}

impl Default for InferenceService {
    fn default() -> Self {
        Self::new()
    }
}

impl InferenceService {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            statuses: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 启动一个推理实例
    pub async fn start_instance(&self, db: &Database, config: InferenceConfig) -> Result<()> {
        // 1. 检查是否已经在运行
        {
            let statuses = self.statuses.lock().await;
            if let Some(status) = statuses.get(&config.model_id) {
                if *status == InstanceStatus::Running || *status == InstanceStatus::Starting {
                    return Ok(()); // Already running
                }
            }
        }

        // 2. 更新状态为 Starting
        self.set_status(&config.model_id, InstanceStatus::Starting)
            .await;

        // 3. 查找 llama-server 可执行文件
        let server_bin = self.find_server_binary().await?;

        // 4. 构建命令
        // llama-server -m <model_path> --port <port> -c <ctx> -ngl <gpu_layers>
        let mut cmd = Command::new(server_bin);
        cmd.arg("-m")
            .arg(&config.file_path)
            .arg("--port")
            .arg(config.port.to_string())
            .arg("-c")
            .arg(config.context_size.to_string())
            .arg("-ngl")
            .arg(config.gpu_layers.to_string())
            // 禁用 web ui，只提供 API
            .arg("--nobrowser")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        tracing::info!("Starting inference: {:?}", cmd);

        match cmd.spawn() {
            Ok(child) => {
                // 保存进程句柄
                let mut processes = self.processes.lock().await;
                processes.insert(config.model_id.clone(), child);

                // 等待健康检查成功后标记为 Running
                let health_check_result = self
                    .wait_for_health_check(&config.model_id, config.port)
                    .await;

                match health_check_result {
                    Ok(()) => {
                        self.set_status(&config.model_id, InstanceStatus::Running)
                            .await;
                    }
                    Err(e) => {
                        // 健康检查失败，移除进程并标记为 Failed
                        processes.remove(&config.model_id);
                        self.set_status(&config.model_id, InstanceStatus::Failed(e.to_string()))
                            .await;
                        return Err(e);
                    }
                }

                // 5. 注册到 Router
                self.register_upstream(db, &config).await?;

                Ok(())
            }
            Err(e) => {
                let err_msg = format!("Failed to spawn process: {}", e);
                self.set_status(&config.model_id, InstanceStatus::Failed(err_msg.clone()))
                    .await;
                Err(InferenceError::ProcessSpawnFailed(err_msg))
            }
        }
    }

    /// 停止一个推理实例
    pub async fn stop_instance(&self, db: &Database, model_id: &str) -> Result<()> {
        let mut processes = self.processes.lock().await;
        if let Some(mut child) = processes.remove(model_id) {
            // 尝试优雅停止
            child
                .kill()
                .await
                .map_err(|e| InferenceError::ProcessKillFailed(e.to_string()))?;
            self.set_status(model_id, InstanceStatus::Stopped).await;

            // 从 Router 注销
            self.unregister_upstream(db, model_id).await?;
        }
        Ok(())
    }

    /// 获取实例状态
    pub async fn get_status(&self, model_id: &str) -> InstanceStatus {
        let statuses = self.statuses.lock().await;
        statuses
            .get(model_id)
            .cloned()
            .unwrap_or(InstanceStatus::Stopped)
    }

    async fn set_status(&self, model_id: &str, status: InstanceStatus) {
        let mut statuses = self.statuses.lock().await;
        statuses.insert(model_id.to_string(), status);
    }

    // 辅助：查找 server 二进制文件
    async fn find_server_binary(&self) -> Result<String> {
        if let Ok(path) = std::env::var("BURNCLOUD_LLAMA_BIN") {
            return Ok(path);
        }

        // Windows 默认查找路径
        let possible_paths = vec![
            "./bin/llama-server.exe",
            "./llama-server.exe",
            "llama-server.exe", // in PATH
        ];

        for path in possible_paths {
            if tokio::fs::try_exists(path).await.unwrap_or(false) {
                return Ok(path.to_string());
            }
        }

        // 如果找不到，返回 "llama-server" 尝试依靠 PATH
        Ok("llama-server".to_string())
    }

    // 注册本地模型到 channel_providers
    async fn register_upstream(&self, db: &Database, config: &InferenceConfig) -> Result<()> {
        let channel_id_str = format!("local-{}", config.model_id);
        let base_url = format!("http://127.0.0.1:{}", config.port);

        // 构建 Channel 对象
        let mut channel = Channel {
            id: 0,               // Will be assigned by database
            type_: 1,            // OpenAI type
            key: "".to_string(), // 无需鉴权
            status: 1,
            name: format!("Local: {}", config.model_id),
            weight: 1,
            created_time: None,
            test_time: None,
            response_time: None,
            base_url: Some(base_url),
            models: config.model_id.clone(),
            group: "default".to_string(),
            used_quota: 0,
            model_mapping: None,
            priority: 100, // 本地模型优先级高
            auto_ban: 1,
            other_info: None,
            tag: Some("local-inference".to_string()),
            setting: None,
            param_override: None,
            header_override: None,
            remark: None,
            api_version: Some("default".to_string()),
            pricing_region: None,
            rpm_cap: None,
            tpm_cap: None,
            reservation_green: None,
            reservation_yellow: None,
            reservation_red: None,
        };

        // 创建 channel
        let channel_id = ChannelProviderModel::create(db, &mut channel)
            .await
            .map_err(|e| {
                InferenceError::ProcessSpawnFailed(format!("Failed to create channel: {}", e))
            })?;

        // 创建 channel_ability
        let ability = ChannelAbilityInput {
            group: "default".to_string(),
            model: config.model_id.clone(),
            channel_id,
            enabled: true,
            priority: 100,
            weight: 1,
        };
        ChannelAbilityModel::create_batch(db, &[ability])
            .await
            .map_err(|e| {
                InferenceError::ProcessSpawnFailed(format!("Failed to create ability: {}", e))
            })?;

        tracing::info!(
            "Registered local channel: {} (ID: {})",
            channel_id_str,
            channel_id
        );
        Ok(())
    }

    async fn unregister_upstream(&self, db: &Database, model_id: &str) -> Result<()> {
        // 查找并删除对应的 channel
        let channels = ChannelProviderModel::list(db, 1000, 0)
            .await
            .map_err(|e| InferenceError::ProcessKillFailed(e.to_string()))?;

        for channel in channels {
            if channel.name == format!("Local: {}", model_id) {
                // 先删除 abilities
                ChannelAbilityModel::delete_by_channel(db, channel.id)
                    .await
                    .map_err(|e| InferenceError::ProcessKillFailed(e.to_string()))?;
                // 再删除 channel
                ChannelProviderModel::delete(db, channel.id)
                    .await
                    .map_err(|e| InferenceError::ProcessKillFailed(e.to_string()))?;
                tracing::info!("Unregistered local channel: {}", model_id);
                break;
            }
        }
        Ok(())
    }

    /// 等待推理服务健康检查通过
    ///
    /// 轮询服务的 `/v1/models` 端点，直到服务就绪或超时
    async fn wait_for_health_check(&self, model_id: &str, port: u16) -> Result<()> {
        let url = format!("http://127.0.0.1:{}/v1/models", port);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| InferenceError::ProcessSpawnFailed(format!("HTTP client error: {}", e)))?;

        // 最多等待 60 秒，每秒检查一次
        let max_attempts = 60;
        let mut attempts = 0;

        while attempts < max_attempts {
            attempts += 1;

            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    tracing::info!(
                        "Health check passed for {} after {} attempts",
                        model_id,
                        attempts
                    );
                    return Ok(());
                }
                Ok(resp) => {
                    // 服务器响应了但返回非成功状态码，可能还在初始化
                    tracing::debug!(
                        "Health check attempt {} for {}: status {}",
                        attempts,
                        model_id,
                        resp.status()
                    );
                }
                Err(e) => {
                    // 连接失败，服务可能还没启动
                    tracing::trace!(
                        "Health check attempt {} for {}: connection error: {}",
                        attempts,
                        model_id,
                        e
                    );
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        Err(InferenceError::ProcessSpawnFailed(format!(
            "Health check timeout for {} after {} seconds",
            model_id, max_attempts
        )))
    }
}
