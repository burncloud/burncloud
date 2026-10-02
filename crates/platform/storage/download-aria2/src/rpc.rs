use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use reqwest::Client;
use serde::{Deserialize, Serialize};
#[allow(clippy::disallowed_types)]
// Value used for aria2 JSON-RPC protocol construction/parsing
use serde_json::Value;
use tokio::sync::Mutex;

use crate::error::{Aria2Error, Aria2Result};
use crate::types::{DownloadOptions, DownloadStatus, FileInfo, GlobalStat};

// ============================================================================
// RPC 客户端
// ============================================================================

/// 表示添加 URI 后任务是新创建的还是已存在的。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddUriOutcome {
    /// aria2 创建了新任务，并返回新任务的 GID。
    Created(String),
    /// aria2 中已存在匹配任务，并返回现有任务的 GID。
    Existing(String),
}

impl AddUriOutcome {
    /// 返回新创建或已存在任务的 GID。
    // 输入：当前添加 URI 的结果。
    // 功能：统一读取两种结果中的任务 GID。
    // 错误：无。
    pub fn gid(&self) -> &str {
        match self {
            Self::Created(gid) | Self::Existing(gid) => gid,
        }
    }

    /// 判断本次调用是否创建了新任务。
    // 输入：当前添加 URI 的结果。
    // 功能：区分新建任务和复用已有任务。
    // 错误：无。
    pub fn is_created(&self) -> bool {
        matches!(self, Self::Created(_))
    }
}

pub struct Aria2RpcClient {
    client: Client,
    base_url: String,
    secret: Option<String>,
    request_id: Arc<AtomicU64>,
    add_uri_lock: Arc<Mutex<()>>,
}

impl Aria2RpcClient {
    // 输入：RPC 端口和可选认证密钥。
    // 功能：创建用于访问指定 aria2 JSON-RPC 服务的客户端。
    // 错误：无，客户端构造不执行网络请求。
    pub fn new(port: u16, secret: Option<String>) -> Self {
        // 为直接创建的客户端初始化独立的添加任务锁
        Self::with_add_uri_lock(port, secret, Arc::new(Mutex::new(())))
    }

    /// 创建使用共享添加任务锁的 RPC 客户端。
    // 输入：RPC 端口、可选认证密钥和同一 aria2 实例共享的添加任务锁。
    // 功能：让不同 RPC 客户端实例复用同一把“查询并添加”互斥锁。
    // 错误：无，客户端构造不执行网络请求。
    pub(crate) fn with_add_uri_lock(
        port: u16,
        secret: Option<String>,
        add_uri_lock: Arc<Mutex<()>>,
    ) -> Self {
        // 初始化 HTTP 客户端、服务地址与请求编号
        Self {
            client: Client::new(),
            base_url: format!("http://localhost:{}/jsonrpc", port),
            secret,
            request_id: Arc::new(AtomicU64::new(1)),
            add_uri_lock,
        }
    }

    /// 调用 aria2 JSON-RPC 方法并将响应结果反序列化为目标类型。
    ///
    /// 参数由 JSON-RPC 参数列表规则展开；HTTP、RPC 或响应格式错误会返回对应错误。
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

        // 数组参数展开为多个 RPC 参数，null 表示没有额外参数。
        match param_value {
            Value::Array(array) => rpc_params.extend(array),
            Value::Null => {}
            other => rpc_params.push(other),
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

        // HTTP 非成功状态可能携带代理或服务端错误正文，先保留状态码和正文。
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Aria2Error::HttpError { status, body });
        }

        // 解析成功 HTTP 响应中的 JSON-RPC 消息。
        #[allow(clippy::disallowed_types)] // Value used for aria2 JSON-RPC response parsing
        let rpc_response: Value = response
            .json()
            .await
            .map_err(|e| Aria2Error::RpcError(e.to_string()))?;

        // 忽略空的 error 字段，仅将实际返回的错误对象作为 RPC 错误。
        if let Some(error) = rpc_response.get("error").filter(|error| !error.is_null()) {
            return Err(Aria2Error::RpcError(format!("服务器错误: {}", error)));
        }

        // 确保服务端响应包含 result 字段，再反序列化调用结果。
        let result = rpc_response
            .get("result")
            .ok_or_else(|| Aria2Error::RpcError("响应缺少 result 字段".into()))?
            .clone();
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
    ) -> Aria2Result<AddUriOutcome> {
        // 空 URI 没有可提交的下载目标，直接返回参数错误。
        if uris.is_empty() {
            return Err(Aria2Error::RpcError(
                "aria2.addUri 的 URI 列表不能为空".to_string(),
            ));
        }

        // 锁住查询到添加的完整过程，避免并发调用同时通过重复任务检查。
        let _add_uri_guard = self.add_uri_lock.lock().await;

        // 检查是否存在相同 URI 和存储路径的活跃任务
        if let Some(existing_gid) = self.find_existing_task(&uris, &options).await? {
            return Ok(AddUriOutcome::Existing(existing_gid));
        }

        // 根据是否提供下载配置选择 RPC 参数形式，并统一提取新任务 GID。
        let gid = if let Some(opts) = options {
            self.call_method("aria2.addUri", (uris, opts)).await?
        } else {
            self.call_method("aria2.addUri", uris).await?
        };

        Ok(AddUriOutcome::Created(gid))
    }

    /// 查找具有相同 URI 和存储路径的活跃任务
    // 输入：待比较的 URI 列表和可选下载配置。
    // 功能：遍历活跃任务以查找重复下载。
    // 错误：任务详情比较失败时返回 RpcError。
    async fn find_existing_task(
        &self,
        uris: &[String],
        options: &Option<DownloadOptions>,
    ) -> Aria2Result<Option<String>> {
        // 仅获取活跃任务，等待和已停止任务不参与重复检查。
        let active_tasks = self.tell_active().await.unwrap_or_default();

        // 检查每个任务
        for task in active_tasks {
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

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::Aria2RpcClient;
    use crate::error::Aria2Error;

    // 启动一次性本地 HTTP 服务，用于验证 RPC 响应处理逻辑。
    async fn start_rpc_server(
        status: &str,
        body: &str,
    ) -> Result<u16, Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );

        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut request = [0; 1024];
                let _ = stream.read(&mut request).await;
                let _ = stream.write_all(response.as_bytes()).await;
            }
        });

        Ok(port)
    }

    /// 验证非成功 HTTP 状态会保留状态码和响应正文。
    #[tokio::test]
    async fn non_success_http_status_returns_http_error(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let port = start_rpc_server("502 Bad Gateway", "upstream unavailable").await?;
        let client = Aria2RpcClient::new(port, None);

        let result = client.call_method::<_, String>("test", ()).await;
        assert!(matches!(
            result,
            Err(Aria2Error::HttpError { status, body })
                if status.as_u16() == 502 && body == "upstream unavailable"
        ));

        Ok(())
    }

    /// 验证 null error 字段不会覆盖有效的 result。
    #[tokio::test]
    async fn null_error_field_allows_result() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    {
        let port = start_rpc_server(
            "200 OK",
            r#"{"jsonrpc":"2.0","id":"1","error":null,"result":"ok"}"#,
        )
        .await?;
        let client = Aria2RpcClient::new(port, None);

        let result: String = client.call_method("test", ()).await?;
        assert_eq!(result, "ok");

        Ok(())
    }

    /// 验证缺少 result 字段时返回明确的 RPC 错误。
    #[tokio::test]
    async fn missing_result_field_returns_rpc_error(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let port = start_rpc_server("200 OK", r#"{"jsonrpc":"2.0","id":"1"}"#).await?;
        let client = Aria2RpcClient::new(port, None);

        let result = client.call_method::<_, String>("test", ()).await;
        assert!(matches!(
            result,
            Err(Aria2Error::RpcError(message)) if message == "响应缺少 result 字段"
        ));

        Ok(())
    }

    /// 验证添加 URI 结果能够区分新建任务和已有任务。
    #[test]
    fn add_uri_outcome_exposes_gid_and_creation_state() {
        let created = super::AddUriOutcome::Created("created-gid".to_string());
        let existing = super::AddUriOutcome::Existing("existing-gid".to_string());

        assert_eq!(created.gid(), "created-gid");
        assert!(created.is_created());
        assert_eq!(existing.gid(), "existing-gid");
        assert!(!existing.is_created());
    }

    /// 验证空 URI 列表会在发起 RPC 请求前返回错误。
    #[tokio::test]
    async fn empty_uri_list_returns_rpc_error() {
        let client = Aria2RpcClient::new(0, None);

        let result = client.add_uri(Vec::new(), None).await;

        assert!(matches!(
            result,
            Err(Aria2Error::RpcError(message))
                if message == "aria2.addUri 的 URI 列表不能为空"
        ));
    }
}
