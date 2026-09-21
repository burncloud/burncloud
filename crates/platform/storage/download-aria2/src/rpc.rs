use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use reqwest::Client;
use serde::{Deserialize, Serialize};
#[allow(clippy::disallowed_types)]
// Value used for aria2 JSON-RPC protocol construction/parsing
use serde_json::Value;

use crate::error::{Aria2Error, Aria2Result};
use crate::types::{DownloadOptions, DownloadStatus, FileInfo, GlobalStat};

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
    // 输入：RPC 端口和可选认证密钥。
    // 功能：创建用于访问指定 aria2 JSON-RPC 服务的客户端。
    // 错误：无，客户端构造不执行网络请求。
    pub fn new(port: u16, secret: Option<String>) -> Self {
        // 初始化 HTTP 客户端、服务地址与请求编号
        Self {
            client: Client::new(),
            base_url: format!("http://localhost:{}/jsonrpc", port),
            secret,
            request_id: Arc::new(AtomicU64::new(1)),
        }
    }

    // 输入：RPC 方法名及其可序列化参数。
    // 功能：构造并发送 JSON-RPC 请求，再将结果反序列化为目标类型。
    // 错误：序列化、请求、响应解析或服务端错误时返回 RpcError。
    async fn call_method<T, R>(&self, method: &str, params: T) -> Aria2Result<R>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        // 初始化 RPC 参数并加入可选认证令牌
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

        // 生成请求编号并构建 JSON-RPC 请求体
        let request_id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": request_id.to_string(),
            "method": method,
            "params": rpc_params
        });

        // 发送 HTTP 请求并读取 JSON-RPC 响应
        let response = self
            .client
            .post(&self.base_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| Aria2Error::RpcError(e.to_string()))?;

        #[allow(clippy::disallowed_types)] // Value used for aria2 JSON-RPC response parsing
        let rpc_response: Value = response
            .json()
            .await
            .map_err(|e| Aria2Error::RpcError(e.to_string()))?;

        // 优先处理服务端返回的 RPC 错误
        if let Some(error) = rpc_response.get("error") {
            return Err(Aria2Error::RpcError(format!("服务器错误: {}", error)));
        }

        // 提取并反序列化 RPC 调用结果
        let result = rpc_response["result"].clone();
        serde_json::from_value(result).map_err(|e| Aria2Error::RpcError(e.to_string()))
    }

    /// 添加 URI 下载任务
    // 输入：待下载 URI 列表和可选的下载配置。
    // 功能：避免重复任务后，通过 aria2.addUri 创建下载任务。
    // 错误：任务查询或 RPC 调用失败时返回 RpcError。
    pub async fn add_uri(
        &self,
        uris: Vec<String>,
        options: Option<DownloadOptions>,
    ) -> Aria2Result<String> {
        // 检查是否存在相同URI和存储路径的任务
        if let Some(existing_gid) = self.find_existing_task(&uris, &options).await? {
            return Ok(existing_gid);
        }

        // 根据是否提供下载配置选择 RPC 参数形式
        if let Some(opts) = options {
            self.call_method("aria2.addUri", (uris, opts)).await
        } else {
            self.call_method("aria2.addUri", uris).await
        }
    }

    /// 查找具有相同URI和存储路径的现有任务
    // 输入：待比较的 URI 列表和可选下载配置。
    // 功能：遍历活跃、等待和已停止任务以查找重复下载。
    // 错误：任务详情比较失败时返回 RpcError。
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

        // 未匹配到重复任务
        Ok(None)
    }

    /// 检查任务是否具有相同的URI和存储路径
    // 输入：现有任务状态、待下载 URI 列表和可选下载配置。
    // 功能：比较任务文件 URI 与目标目录，判断是否为相同任务。
    // 错误：获取任务文件信息失败时被忽略，其他比较错误返回 RpcError。
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

        // 没有发现 URI 与目录均匹配的任务
        Ok(false)
    }

    /// 获取下载状态
    // 输入：下载任务的 GID。
    // 功能：调用 aria2.tellStatus 获取单个任务状态。
    // 错误：RPC 请求或响应解析失败时返回 RpcError。
    pub async fn tell_status(&self, gid: &str) -> Aria2Result<DownloadStatus> {
        // 转发状态查询到通用 RPC 调用器
        self.call_method("aria2.tellStatus", gid).await
    }

    /// 获取活跃下载列表
    // 输入：无。
    // 功能：调用 aria2.tellActive 获取所有活跃下载。
    // 错误：RPC 请求或响应解析失败时返回 RpcError。
    pub async fn tell_active(&self) -> Aria2Result<Vec<DownloadStatus>> {
        // 转发活跃任务查询到通用 RPC 调用器
        self.call_method("aria2.tellActive", ()).await
    }

    /// 获取等待下载列表
    // 输入：任务偏移量和返回数量。
    // 功能：调用 aria2.tellWaiting 获取等待下载列表。
    // 错误：RPC 请求或响应解析失败时返回 RpcError。
    pub async fn tell_waiting(&self, offset: u32, num: u32) -> Aria2Result<Vec<DownloadStatus>> {
        // 转发等待任务查询到通用 RPC 调用器
        self.call_method("aria2.tellWaiting", (offset, num)).await
    }

    /// 获取已停止下载列表
    // 输入：任务偏移量和返回数量。
    // 功能：调用 aria2.tellStopped 获取已停止下载列表。
    // 错误：RPC 请求或响应解析失败时返回 RpcError。
    pub async fn tell_stopped(&self, offset: u32, num: u32) -> Aria2Result<Vec<DownloadStatus>> {
        // 转发停止任务查询到通用 RPC 调用器
        self.call_method("aria2.tellStopped", (offset, num)).await
    }

    /// 获取下载文件信息
    // 输入：下载任务的 GID。
    // 功能：调用 aria2.getFiles 获取任务关联的文件信息。
    // 错误：RPC 请求或响应解析失败时返回 RpcError。
    pub async fn get_files(&self, gid: &str) -> Aria2Result<Vec<FileInfo>> {
        // 转发文件信息查询到通用 RPC 调用器
        self.call_method("aria2.getFiles", gid).await
    }

    /// 获取全局统计信息
    // 输入：无。
    // 功能：调用 aria2.getGlobalStat 获取全局下载统计。
    // 错误：RPC 请求或响应解析失败时返回 RpcError。
    pub async fn get_global_stat(&self) -> Aria2Result<GlobalStat> {
        // 转发全局统计查询到通用 RPC 调用器
        self.call_method("aria2.getGlobalStat", ()).await
    }

    /// 暂停下载
    // 输入：下载任务的 GID。
    // 功能：调用 aria2.pause 暂停指定下载任务。
    // 错误：RPC 请求或响应解析失败时返回 RpcError。
    pub async fn pause(&self, gid: &str) -> Aria2Result<String> {
        // 转发暂停请求到通用 RPC 调用器
        self.call_method("aria2.pause", gid).await
    }

    /// 恢复下载
    // 输入：下载任务的 GID。
    // 功能：调用 aria2.unpause 恢复指定下载任务。
    // 错误：RPC 请求或响应解析失败时返回 RpcError。
    pub async fn unpause(&self, gid: &str) -> Aria2Result<String> {
        // 转发恢复请求到通用 RPC 调用器
        self.call_method("aria2.unpause", gid).await
    }

    /// 移除下载
    // 输入：下载任务的 GID。
    // 功能：调用 aria2.remove 移除指定下载任务。
    // 错误：RPC 请求或响应解析失败时返回 RpcError。
    pub async fn remove(&self, gid: &str) -> Aria2Result<String> {
        // 转发移除请求到通用 RPC 调用器
        self.call_method("aria2.remove", gid).await
    }

    /// 关闭 aria2
    // 输入：无。
    // 功能：调用 aria2.shutdown 请求关闭 aria2 服务。
    // 错误：RPC 请求或响应解析失败时返回 RpcError。
    pub async fn shutdown(&self) -> Aria2Result<String> {
        // 转发关闭请求到通用 RPC 调用器
        self.call_method("aria2.shutdown", ()).await
    }
}
