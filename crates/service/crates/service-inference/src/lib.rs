//! # BurnCloud Service Inference
//!
//! 本地推理服务管理模块，负责 `llama-server` 等推理后端的进程管理。

use std::process::Stdio;
use tokio::process::{Command, Child};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use anyhow::{Result, Context};
use burncloud_database::Database;
use burncloud_database_router::{RouterDatabase, DbUpstream};

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
    // 数据库连接
    db: Database,
}

impl InferenceService {
    pub async fn new() -> Result<Self> {
        let db = Database::new().await?;
        // Ensure tables exist
        RouterDatabase::init(&db).await?;
        
        Ok(Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            statuses: Arc::new(Mutex::new(HashMap::new())),
            db,
        })
    }

    /// 启动一个推理实例
    pub async fn start_instance(&self, config: InferenceConfig) -> Result<()> {
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
        self.set_status(&config.model_id, InstanceStatus::Starting).await;

        // 3. 查找 llama-server 可执行文件
        let server_bin = self.find_server_binary().await?;

        // 4. 构建命令
        // llama-server -m <model_path> --port <port> -c <ctx> -ngl <gpu_layers>
        let mut cmd = Command::new(server_bin);
        cmd.arg("-m").arg(&config.file_path)
           .arg("--port").arg(config.port.to_string())
           .arg("-c").arg(config.context_size.to_string())
           .arg("-ngl").arg(config.gpu_layers.to_string())
           // 禁用 web ui，只提供 API
           .arg("--nobrowser") 
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        println!("Starting inference: {:?}", cmd);

        match cmd.spawn() {
            Ok(child) => {
                // 保存进程句柄
                let mut processes = self.processes.lock().await;
                processes.insert(config.model_id.clone(), child);
                
                // 更新状态为 Running
                // TODO: 这里应该等待 health check 成功才标记为 Running
                self.set_status(&config.model_id, InstanceStatus::Running).await;

                // 5. 注册到 Router
                self.register_upstream(&config).await?;
                
                Ok(())
            }
            Err(e) => {
                let err_msg = format!("Failed to spawn process: {}", e);
                self.set_status(&config.model_id, InstanceStatus::Failed(err_msg.clone())).await;
                Err(anyhow::anyhow!(err_msg))
            }
        }
    }

    /// 停止一个推理实例
    pub async fn stop_instance(&self, model_id: &str) -> Result<()> {
        let mut processes = self.processes.lock().await;
        if let Some(mut child) = processes.remove(model_id) {
            // 尝试优雅停止
            child.kill().await.context("Failed to kill process")?;
            self.set_status(model_id, InstanceStatus::Stopped).await;
            
            // 从 Router 注销
            self.unregister_upstream(model_id).await?;
        }
        Ok(())
    }

    /// 获取实例状态
    pub async fn get_status(&self, model_id: &str) -> InstanceStatus {
        let statuses = self.statuses.lock().await;
        statuses.get(model_id).cloned().unwrap_or(InstanceStatus::Stopped)
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

    // 注册 Upstream 到 Router 数据库
    async fn register_upstream(&self, config: &InferenceConfig) -> Result<()> {
        let upstream_id = format!("local-{}", config.model_id);
        let base_url = format!("http://127.0.0.1:{}", config.port);
        
        // 构建 Upstream 对象
        let upstream = DbUpstream {
            id: upstream_id.clone(),
            name: format!("Local: {}", config.model_id),
            base_url,
            api_key: "".to_string(), // 无需鉴权
            match_path: "/v1/chat/completions".to_string(), // 默认 OpenAI 兼容路径
            auth_type: "Bearer".to_string(), // 占位，实际上不需要，但 Router 需要非空
            priority: 100, // 本地模型优先级高
            protocol: "openai".to_string(),
        };

        // Upsert: 先删后插，或者检查是否存在
        // 这里简单处理：DELETE 然后 INSERT，确保是最新的
        let _ = RouterDatabase::delete_upstream(&self.db, &upstream_id).await;
        RouterDatabase::create_upstream(&self.db, &upstream).await?;
        
        println!("Registered local upstream: {}", upstream_id);
        Ok(())
    }

    async fn unregister_upstream(&self, model_id: &str) -> Result<()> {
        let upstream_id = format!("local-{}", model_id);
        RouterDatabase::delete_upstream(&self.db, &upstream_id).await?;
        println!("Unregistered local upstream: {}", upstream_id);
        Ok(())
    }
}
