//! # BurnCloud Aria2 下载库
//!
//! 这是一个简单的 Rust 库，用于下载、配置和管理 aria2 下载器。
//! 遵循"极度简单"的设计原则，所有功能都在此文件中实现。

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// 常量定义
const DEFAULT_PORT: u16 = 6800;
const MAX_PORT_RANGE: u16 = 100;
const ARIA2_MAIN_URL: &str = "https://github.com/aria2/aria2/releases/download/release-1.37.0/aria2-1.37.0-win-64bit-build1.zip";
const ARIA2_BACKUP_URL: &str =
    "https://gitee.com/burncloud/aria2/raw/master/aria2-1.37.0-win-64bit-build1.zip";

/// 获取 BurnCloud 目录路径
fn get_burncloud_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .map(|profile| {
            PathBuf::from(profile)
                .join("AppData")
                .join("Local")
                .join("BurnCloud")
        })
        .unwrap_or_else(|_| PathBuf::from(r"C:\Users\Default\AppData\Local\BurnCloud"))
}

// ============================================================================
// 错误类型定义
// ============================================================================

#[derive(Debug)]
pub enum Aria2Error {
    DownloadError(String),
    PortError(String),
    RpcError(String),
    DaemonError(String),
    ProcessError(String),
    ConfigError(String),
}

impl std::fmt::Display for Aria2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Aria2Error::DownloadError(msg) => write!(f, "下载错误: {}", msg),
            Aria2Error::PortError(msg) => write!(f, "端口错误: {}", msg),
            Aria2Error::RpcError(msg) => write!(f, "RPC错误: {}", msg),
            Aria2Error::DaemonError(msg) => write!(f, "守护进程错误: {}", msg),
            Aria2Error::ProcessError(msg) => write!(f, "进程错误: {}", msg),
            Aria2Error::ConfigError(msg) => write!(f, "配置错误: {}", msg),
        }
    }
}

impl std::error::Error for Aria2Error {}

pub type Aria2Result<T> = Result<T, Aria2Error>;

// ============================================================================
// 数据结构定义
// ============================================================================

#[derive(Debug, Clone)]
pub struct Aria2Config {
    pub port: u16,
    pub secret: Option<String>,
    pub download_dir: PathBuf,
    pub max_connections: u8,
    pub split_size: String,
    pub aria2_path: PathBuf,
}

impl Default for Aria2Config {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            secret: None,
            download_dir: std::env::current_dir()
                .unwrap_or_default()
                .join("downloads"),
            max_connections: 16,
            split_size: "1M".to_string(),
            aria2_path: get_burncloud_dir().join("aria2c.exe"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split: Option<u8>,
    #[serde(
        rename = "max-connection-per-server",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_connection_per_server: Option<u8>,
    #[serde(rename = "continue", skip_serializing_if = "Option::is_none")]
    pub continue_download: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadStatus {
    pub gid: String,
    pub status: String,
    #[serde(rename = "totalLength")]
    pub total_length: String,
    #[serde(rename = "completedLength")]
    pub completed_length: String,
    #[serde(rename = "downloadSpeed")]
    pub download_speed: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GlobalStat {
    #[serde(rename = "downloadSpeed")]
    pub download_speed: String,
    #[serde(rename = "numActive")]
    pub num_active: String,
    #[serde(rename = "numWaiting")]
    pub num_waiting: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub uris: Vec<UriInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UriInfo {
    pub uri: String,
    pub status: String,
}

pub struct Aria2Instance {
    pub process: Child,
    pub port: u16,
    pub config: Aria2Config,
}

impl Aria2Instance {
    pub fn is_running(&mut self) -> bool {
        matches!(self.process.try_wait(), Ok(None))
    }

    pub fn kill(&mut self) -> Aria2Result<()> {
        self.process
            .kill()
            .map_err(|e| Aria2Error::ProcessError(e.to_string()))?;
        self.process
            .wait()
            .map_err(|e| Aria2Error::ProcessError(e.to_string()))?;
        Ok(())
    }
}

// ============================================================================
// Aria2 下载功能
// ============================================================================

/// 下载 aria2 二进制文件
pub async fn download_aria2() -> Aria2Result<PathBuf> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| Aria2Error::DownloadError(e.to_string()))?;

    let target_dir = get_burncloud_dir();
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| Aria2Error::DownloadError(format!("创建目录失败: {}", e)))?;

    let zip_path = target_dir.join("aria2.zip");
    let exe_path = target_dir.join("aria2c.exe");

    // 如果 exe 已存在，直接返回
    if exe_path.exists() {
        return Ok(exe_path);
    }

    // 尝试主链接下载
    match download_file(&client, ARIA2_MAIN_URL, &zip_path).await {
        Ok(_) => println!("从主链接下载成功"),
        Err(_) => {
            println!("主链接下载失败，尝试备用链接...");
            download_file(&client, ARIA2_BACKUP_URL, &zip_path)
                .await
                .map_err(|e| Aria2Error::DownloadError(format!("所有下载链接均失败: {}", e)))?;
            println!("从备用链接下载成功");
        }
    }

    // 解压 ZIP 文件
    extract_aria2(&zip_path, &target_dir)?;

    // 删除 ZIP 文件
    let _ = std::fs::remove_file(&zip_path);

    if exe_path.exists() {
        Ok(exe_path)
    } else {
        Err(Aria2Error::DownloadError(
            "解压后未找到 aria2c.exe".to_string(),
        ))
    }
}

async fn download_file(client: &Client, url: &str, path: &Path) -> Aria2Result<()> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| Aria2Error::DownloadError(e.to_string()))?;

    if !response.status().is_success() {
        return Err(Aria2Error::DownloadError(format!(
            "HTTP错误: {}",
            response.status()
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| Aria2Error::DownloadError(e.to_string()))?;

    std::fs::write(path, &bytes).map_err(|e| Aria2Error::DownloadError(e.to_string()))?;

    Ok(())
}

fn extract_aria2(zip_path: &Path, target_dir: &Path) -> Aria2Result<()> {
    let file =
        std::fs::File::open(zip_path).map_err(|e| Aria2Error::DownloadError(e.to_string()))?;

    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| Aria2Error::DownloadError(e.to_string()))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| Aria2Error::DownloadError(e.to_string()))?;

        if file.name().ends_with("aria2c.exe") {
            let mut out_file = std::fs::File::create(target_dir.join("aria2c.exe"))
                .map_err(|e| Aria2Error::DownloadError(e.to_string()))?;
            std::io::copy(&mut file, &mut out_file)
                .map_err(|e| Aria2Error::DownloadError(e.to_string()))?;
            return Ok(());
        }
    }

    Err(Aria2Error::DownloadError(
        "ZIP文件中未找到 aria2c.exe".to_string(),
    ))
}

// ============================================================================
// 端口管理
// ============================================================================

/// 检查端口是否可用
pub fn check_port_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// 查找可用端口
pub fn find_available_port() -> Aria2Result<u16> {
    for port in DEFAULT_PORT..=(DEFAULT_PORT + MAX_PORT_RANGE) {
        if check_port_available(port) {
            return Ok(port);
        }
    }
    Err(Aria2Error::PortError("未找到可用端口".to_string()))
}

/// 终止所有aria2c.exe进程
pub fn kill_existing_aria2() {
    let _ = Command::new("taskkill")
        .args(["/F", "/IM", "aria2c.exe"])
        .output();
}

/// 启动 aria2 RPC 服务
pub async fn start_aria2_rpc(config: &Aria2Config) -> Aria2Result<Aria2Instance> {
    // 先终止现有的aria2c.exe进程
    kill_existing_aria2();

    let port = find_available_port()?;

    let mut cmd = Command::new(&config.aria2_path);
    cmd.args([
        "--enable-rpc",
        "--rpc-listen-all",
        &format!("--rpc-listen-port={}", port),
        &format!("--dir={}", config.download_dir.display()),
        &format!("--max-connection-per-server={}", config.max_connections),
        &format!("--split={}", config.max_connections),
        &format!("--min-split-size={}", config.split_size),
        "--continue=true",
        "--max-tries=0",
        "--retry-wait=3",
        "--daemon=true",
    ]);

    if let Some(secret) = &config.secret {
        cmd.arg(format!("--rpc-secret={}", secret));
    }

    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Aria2Error::ProcessError(e.to_string()))?;

    let instance = Aria2Instance {
        process: child,
        port,
        config: config.clone(),
    };

    // 等待 RPC 服务启动
    wait_for_rpc_ready(port, &config.secret).await?;

    Ok(instance)
}

async fn wait_for_rpc_ready(port: u16, secret: &Option<String>) -> Aria2Result<()> {
    let client = Client::new();
    let url = format!("http://localhost:{}/jsonrpc", port);

    for _ in 0..30 {
        let mut params = vec![];
        if let Some(s) = secret {
            params.push(Value::String(format!("token:{}", s)));
        }

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "test",
            "method": "aria2.getVersion",
            "params": params
        });

        if let Ok(response) = client.post(&url).json(&request).send().await {
            if response.status().is_success() {
                return Ok(());
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    Err(Aria2Error::RpcError("RPC 服务启动超时".to_string()))
}

// ============================================================================
// RPC 客户端
// ============================================================================

pub struct Aria2RpcClient {
    client: Client,
    base_url: String,
    secret: Option<String>,
    request_id: Arc<AtomicU64>,
}

impl Aria2RpcClient {
    pub fn new(port: u16, secret: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: format!("http://localhost:{}/jsonrpc", port),
            secret,
            request_id: Arc::new(AtomicU64::new(1)),
        }
    }

    async fn call_method<T, R>(&self, method: &str, params: T) -> Aria2Result<R>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let mut rpc_params = Vec::new();

        // 添加 secret（如果配置了）
        if let Some(secret) = &self.secret {
            rpc_params.push(Value::String(format!("token:{}", secret)));
        }

        // 添加其他参数
        let param_value =
            serde_json::to_value(&params).map_err(|e| Aria2Error::RpcError(e.to_string()))?;

        // 如果参数是数组，则展开每个元素作为单独的参数
        if let Value::Array(array) = param_value {
            rpc_params.extend(array);
        } else if !param_value.is_null() {
            rpc_params.push(param_value);
        }

        let request_id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": request_id.to_string(),
            "method": method,
            "params": rpc_params
        });

        let response = self
            .client
            .post(&self.base_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| Aria2Error::RpcError(e.to_string()))?;

        let rpc_response: Value = response
            .json()
            .await
            .map_err(|e| Aria2Error::RpcError(e.to_string()))?;

        if let Some(error) = rpc_response.get("error") {
            return Err(Aria2Error::RpcError(format!("服务器错误: {}", error)));
        }

        let result = rpc_response["result"].clone();
        serde_json::from_value(result).map_err(|e| Aria2Error::RpcError(e.to_string()))
    }

    /// 添加 URI 下载任务
    pub async fn add_uri(
        &self,
        uris: Vec<String>,
        options: Option<DownloadOptions>,
    ) -> Aria2Result<String> {
        // 检查是否存在相同URI和存储路径的任务
        if let Some(existing_gid) = self.find_existing_task(&uris, &options).await? {
            return Ok(existing_gid);
        }

        if let Some(opts) = options {
            self.call_method("aria2.addUri", (uris, opts)).await
        } else {
            self.call_method("aria2.addUri", uris).await
        }
    }

    /// 查找具有相同URI和存储路径的现有任务
    async fn find_existing_task(
        &self,
        uris: &[String],
        options: &Option<DownloadOptions>,
    ) -> Aria2Result<Option<String>> {
        // 获取所有任务（活跃、等待、已停止）
        let mut all_tasks = Vec::new();

        // 获取活跃任务
        if let Ok(active) = self.tell_active().await {
            all_tasks.extend(active);
        }

        // 获取等待任务
        if let Ok(waiting) = self.tell_waiting(0, 1000).await {
            all_tasks.extend(waiting);
        }

        // 获取已停止任务
        if let Ok(stopped) = self.tell_stopped(0, 1000).await {
            all_tasks.extend(stopped);
        }

        // 检查每个任务
        for task in all_tasks {
            if let Ok(status) = self.tell_status(&task.gid).await {
                if self.is_same_task(&status, uris, options).await? {
                    return Ok(Some(task.gid));
                }
            }
        }

        Ok(None)
    }

    /// 检查任务是否具有相同的URI和存储路径
    async fn is_same_task(
        &self,
        status: &DownloadStatus,
        uris: &[String],
        options: &Option<DownloadOptions>,
    ) -> Aria2Result<bool> {
        // 获取详细信息需要调用其他方法，这里简化比较
        // 实际实现中可能需要调用 aria2.getFiles 等方法获取完整信息

        // 比较URI（简化版本，实际可能需要更复杂的逻辑）
        if let Ok(files) = self.get_files(&status.gid).await {
            for file in files {
                for uri in uris {
                    if file.uris.iter().any(|u| u.uri == *uri) {
                        // 比较存储路径
                        let target_dir = options.as_ref().and_then(|o| o.dir.as_ref());
                        if let Some(dir) = target_dir {
                            if file.path.starts_with(dir) {
                                return Ok(true);
                            }
                        } else {
                            // 如果没有指定目录，认为是相同的（使用默认目录）
                            return Ok(true);
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// 获取下载状态
    pub async fn tell_status(&self, gid: &str) -> Aria2Result<DownloadStatus> {
        self.call_method("aria2.tellStatus", gid).await
    }

    /// 获取活跃下载列表
    pub async fn tell_active(&self) -> Aria2Result<Vec<DownloadStatus>> {
        self.call_method("aria2.tellActive", ()).await
    }

    /// 获取等待下载列表
    pub async fn tell_waiting(&self, offset: u32, num: u32) -> Aria2Result<Vec<DownloadStatus>> {
        self.call_method("aria2.tellWaiting", (offset, num)).await
    }

    /// 获取已停止下载列表
    pub async fn tell_stopped(&self, offset: u32, num: u32) -> Aria2Result<Vec<DownloadStatus>> {
        self.call_method("aria2.tellStopped", (offset, num)).await
    }

    /// 获取下载文件信息
    pub async fn get_files(&self, gid: &str) -> Aria2Result<Vec<FileInfo>> {
        self.call_method("aria2.getFiles", gid).await
    }

    /// 获取全局统计信息
    pub async fn get_global_stat(&self) -> Aria2Result<GlobalStat> {
        self.call_method("aria2.getGlobalStat", ()).await
    }

    /// 暂停下载
    pub async fn pause(&self, gid: &str) -> Aria2Result<String> {
        self.call_method("aria2.pause", gid).await
    }

    /// 恢复下载
    pub async fn unpause(&self, gid: &str) -> Aria2Result<String> {
        self.call_method("aria2.unpause", gid).await
    }

    /// 移除下载
    pub async fn remove(&self, gid: &str) -> Aria2Result<String> {
        self.call_method("aria2.remove", gid).await
    }

    /// 关闭 aria2
    pub async fn shutdown(&self) -> Aria2Result<String> {
        self.call_method("aria2.shutdown", ()).await
    }
}

// ============================================================================
// 简单守护进程
// ============================================================================

pub struct Aria2Daemon {
    instance: Arc<Mutex<Option<Aria2Instance>>>,
    config: Aria2Config,
    is_running: Arc<AtomicBool>,
}

impl Aria2Daemon {
    pub fn new(config: Aria2Config) -> Self {
        Self {
            instance: Arc::new(Mutex::new(None)),
            config,
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start(&mut self) -> Aria2Result<()> {
        if self.is_running.load(Ordering::SeqCst) {
            return Err(Aria2Error::DaemonError("守护进程已在运行".to_string()));
        }

        let instance = start_aria2_rpc(&self.config).await?;
        println!("aria2 RPC 服务已启动在端口: {}", instance.port);

        *self.instance.lock().unwrap() = Some(instance);
        self.is_running.store(true, Ordering::SeqCst);

        // 启动监控任务
        let instance = Arc::clone(&self.instance);
        let is_running = Arc::clone(&self.is_running);
        let config = self.config.clone();

        tokio::spawn(async move {
            while is_running.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(1000)).await;

                let need_restart = {
                    let mut lock = instance.lock().unwrap();
                    match lock.as_mut() {
                        Some(inst) => !inst.is_running(), // 检查进程是否还在运行
                        None => true,
                    }
                };

                if need_restart {
                    println!("检测到aria2已退出，重启中...");
                    if let Ok(new_instance) = start_aria2_rpc(&config).await {
                        let new_port = new_instance.port;
                        *instance.lock().unwrap() = Some(new_instance);
                        println!("aria2重启成功，端口: {}", new_port);
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);

        if let Some(ref mut instance) = self.instance.lock().unwrap().as_mut() {
            let _ = instance.kill();
        }

        *self.instance.lock().unwrap() = None;
        println!("aria2 守护进程已停止");
    }

    pub fn get_rpc_client(&self) -> Option<Aria2RpcClient> {
        let lock = self.instance.lock().unwrap();
        lock.as_ref()
            .map(|instance| Aria2RpcClient::new(instance.port, self.config.secret.clone()))
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

// ============================================================================
// 统一管理器 - 主要入口点
// ============================================================================

pub struct Aria2Manager {
    daemon: Option<Aria2Daemon>,
    config: Aria2Config,
}

impl Aria2Manager {
    pub fn new() -> Self {
        Self {
            daemon: None,
            config: Aria2Config::default(),
        }
    }

    pub fn with_config(config: Aria2Config) -> Self {
        Self {
            daemon: None,
            config,
        }
    }

    /// 下载并设置 aria2
    pub async fn download_and_setup(&mut self) -> Aria2Result<()> {
        println!("正在下载 aria2...");
        let aria2_path = download_aria2().await?;
        println!("aria2 已下载到: {:?}", aria2_path);

        self.config.aria2_path = aria2_path;
        Ok(())
    }

    /// 启动守护进程
    pub async fn start_daemon(&mut self) -> Aria2Result<()> {
        if self.daemon.is_some() {
            return Err(Aria2Error::DaemonError("守护进程已存在".to_string()));
        }

        let mut daemon = Aria2Daemon::new(self.config.clone());
        daemon.start().await?;
        self.daemon = Some(daemon);

        println!("aria2 守护进程启动成功！");
        Ok(())
    }

    /// 获取 RPC 客户端
    pub fn get_rpc_client(&self) -> Option<&Aria2RpcClient> {
        // 由于借用检查器限制，这里简化实现
        None
    }

    /// 创建新的 RPC 客户端
    pub fn create_rpc_client(&self) -> Option<Aria2RpcClient> {
        self.daemon.as_ref().and_then(|d| d.get_rpc_client())
    }

    /// 关闭管理器
    pub async fn shutdown(&mut self) -> Aria2Result<()> {
        if let Some(ref mut daemon) = self.daemon {
            daemon.stop().await;
        }
        self.daemon = None;
        println!("Aria2Manager 已关闭");
        Ok(())
    }

    /// 检查是否运行中
    pub fn is_running(&self) -> bool {
        self.daemon.as_ref().is_some_and(|d| d.is_running())
    }
}

impl Default for Aria2Manager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 便利函数
// ============================================================================

/// 快速启动 aria2 管理器
pub async fn quick_start() -> Aria2Result<Aria2Manager> {
    let mut manager = Aria2Manager::new();
    manager.download_and_setup().await?;
    manager.start_daemon().await?;
    Ok(manager)
}
