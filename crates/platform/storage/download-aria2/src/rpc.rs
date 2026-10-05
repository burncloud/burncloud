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
#[allow(
    clippy::disallowed_types,
    clippy::panic_in_result_fn,
    clippy::let_underscore_must_use,
    reason = "Three test-only allowances. `serde_json::Value` is how these tests read a JSON-RPC message off the \
              wire, which is an untrusted document by definition -- a test server for that protocol is a protocol \
              boundary. `panic_in_result_fn` fires on `panic!` inside a `#[tokio::test]` that returns `Result`, \
              which is how a test reports a wrong result instead of trusting the function under test to notice. \
              `let_underscore_must_use` fires on `let _: String = client.call_method(..)` inside a loop, where \
              the point is to drive the call and the value is deliberately unused. The production paths in this \
              file carry their own narrower allowances."
)]
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

    /// Read one complete HTTP request and return its body.
    ///
    /// **The existing helper reads a fixed 1024 bytes and does not look at what arrived**, which is enough to
    /// answer "what does the client do with this response" and not enough to answer "what did the client send".
    /// The plan's first item is the JSON-RPC request contract -- `method`, `id` and the parameter order -- so the
    /// request body has to be read properly: headers first, then exactly `Content-Length` bytes.
    ///
    /// A single `read` is not a complete message: it can return part of the headers, and on a loopback socket it
    /// usually returns everything at once only because the request is small. Reading until the header terminator
    /// and then for a counted number of bytes is what makes the assertions below reliable.
    async fn read_request_body(
        stream: &mut tokio::net::TcpStream,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut buffer = Vec::new();
        let mut chunk = [0_u8; 512];

        // Read until the end of the headers.
        let header_end = loop {
            if let Some(position) = find_subslice(&buffer, b"\r\n\r\n") {
                break position + 4;
            }
            let read = stream.read(&mut chunk).await?;
            if read == 0 {
                return Err("the connection closed before the headers ended".into());
            }
            buffer.extend_from_slice(&chunk[..read]);
        };

        // `Content-Length` decides how much body to wait for.
        let headers = String::from_utf8_lossy(&buffer[..header_end]).to_string();
        let length: usize = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse().ok()
                } else {
                    None
                }
            })
            .ok_or("the request had no Content-Length")?;

        while buffer.len() < header_end + length {
            let read = stream.read(&mut chunk).await?;
            if read == 0 {
                return Err("the connection closed before the body ended".into());
            }
            buffer.extend_from_slice(&chunk[..read]);
        }

        Ok(String::from_utf8_lossy(&buffer[header_end..header_end + length]).to_string())
    }

    /// The offset of `needle` in `haystack`, if present.
    fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    /// Write one HTTP response and close.
    async fn write_response(stream: &mut tokio::net::TcpStream, body: &str) -> std::io::Result<()> {
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await
    }

    /// A server that answers each request using `respond`, recording the bodies it received.
    ///
    /// `respond` is given the parsed request and the zero-based call index, so a test can answer differently to
    /// the second call -- which is what the duplicate-detection path needs, since `add_uri` calls
    /// `tellActive`, then `tellStatus` and `getFiles` per task.
    async fn start_scripted_server<F>(
        calls: usize,
        respond: F,
    ) -> Result<
        (
            u16,
            std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
        ),
        Box<dyn std::error::Error + Send + Sync>,
    >
    where
        F: Fn(usize, &serde_json::Value) -> String + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let recorded = std::sync::Arc::clone(&seen);
        let respond = std::sync::Arc::new(respond);

        tokio::spawn(async move {
            for index in 0..calls {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let body = match read_request_body(&mut stream).await {
                    Ok(body) => body,
                    Err(_) => continue,
                };
                let parsed: serde_json::Value =
                    serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
                if let Ok(mut guard) = recorded.lock() {
                    guard.push(parsed.clone());
                }
                let response = respond(index, &parsed);
                let _ = write_response(&mut stream, &response).await;
            }
        });

        Ok((port, seen))
    }

    /// Verify that a non-success HTTP status preserves the status code and the response body.
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

    /// Verify that a null error field does not override a valid result.
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

    /// Verify that a missing result field returns an explicit RPC error.
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

    /// Verify that the added URI result distinguishes newly created and existing tasks.
    #[test]
    fn add_uri_outcome_exposes_gid_and_creation_state() {
        let created = super::AddUriOutcome::Created("created-gid".to_string());
        let existing = super::AddUriOutcome::Existing("existing-gid".to_string());

        assert_eq!(created.gid(), "created-gid");
        assert!(created.is_created());
        assert_eq!(existing.gid(), "existing-gid");
        assert!(!existing.is_created());
    }

    /// Verify that an empty URI list returns an error before making an RPC request.
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

    // ===================================================================================
    // The request contract: method, id, and the parameter order
    // ===================================================================================

    /// The token is the **first** parameter, the method is named, and `jsonrpc` is `2.0`.
    ///
    /// aria2 requires the secret as the first element of `params`; in any other position it is treated as an
    /// ordinary argument and the call is rejected as unauthorised. The body is read off the wire, so this asserts
    /// what was sent rather than what the builder intended.
    #[tokio::test]
    async fn the_request_names_the_method_and_puts_the_token_first(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (port, seen) = start_scripted_server(1, |_index, _request| {
            r#"{"jsonrpc":"2.0","id":"1","result":"ok"}"#.to_string()
        })
        .await?;

        let client = Aria2RpcClient::new(port, Some("s3cr3t".to_string()));
        let _: String = client.call_method("aria2.tellStatus", "gid-1").await?;

        let requests = seen.lock().expect("the recorder is not poisoned").clone();
        let request = requests.first().expect("one request arrived");
        println!("request: {request}");

        assert_eq!(request["jsonrpc"], "2.0");
        assert_eq!(request["method"], "aria2.tellStatus", "the method is named");

        let params = request["params"].as_array().expect("params is an array");
        assert_eq!(params.len(), 2, "the token and the gid: {params:?}");
        assert_eq!(
            params[0], "token:s3cr3t",
            "**the token must be the first parameter**, or aria2 rejects the call as unauthorised"
        );
        assert_eq!(
            params[1], "gid-1",
            "and the method's own argument follows it"
        );

        Ok(())
    }

    /// No token is sent when none is configured, so an unauthenticated aria2 receives no bogus `token:`.
    #[tokio::test]
    async fn no_token_is_sent_when_the_client_has_no_secret(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (port, seen) = start_scripted_server(1, |_index, _request| {
            r#"{"jsonrpc":"2.0","id":"1","result":"ok"}"#.to_string()
        })
        .await?;

        let client = Aria2RpcClient::new(port, None);
        let _: String = client.call_method("aria2.tellStatus", "gid-1").await?;

        let requests = seen.lock().expect("not poisoned").clone();
        let params = requests[0]["params"].as_array().expect("an array").clone();
        println!("params without a secret: {params:?}");
        assert_eq!(params.len(), 1, "only the method's own argument");
        assert!(
            params.iter().all(|p| !p.to_string().contains("token:")),
            "no empty `token:` is sent: {params:?}"
        );

        Ok(())
    }

    /// The request id is a **distinct value per call**, which is what lets a response be matched to its request.
    #[tokio::test]
    async fn each_request_carries_a_distinct_id(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (port, seen) = start_scripted_server(3, |_index, _request| {
            r#"{"jsonrpc":"2.0","id":"1","result":"ok"}"#.to_string()
        })
        .await?;

        let client = Aria2RpcClient::new(port, None);
        for _ in 0..3 {
            let _: String = client.call_method("aria2.getVersion", ()).await?;
        }

        let requests = seen.lock().expect("not poisoned").clone();
        assert_eq!(requests.len(), 3, "three calls reached the server");

        let ids: Vec<String> = requests
            .iter()
            .map(|r| r["id"].as_str().expect("the id is a string").to_string())
            .collect();
        println!("ids: {ids:?}");

        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "the ids must be distinct: {ids:?}");
        assert!(ids.iter().all(|id| !id.is_empty()), "and present: {ids:?}");

        Ok(())
    }

    /// A method with no arguments sends an **empty** parameter list, not a missing one.
    ///
    /// aria2 accepts `"params":[]` for `tellActive` and `getGlobalStat`. Omitting the field, or sending `null`,
    /// is a different request, so this pins the empty-array shape.
    #[tokio::test]
    async fn a_method_without_arguments_sends_an_empty_parameter_list(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (port, seen) = start_scripted_server(1, |_index, _request| {
            r#"{"jsonrpc":"2.0","id":"1","result":"ok"}"#.to_string()
        })
        .await?;

        let client = Aria2RpcClient::new(port, None);
        let _: String = client.call_method("aria2.getGlobalStat", ()).await?;

        let requests = seen.lock().expect("not poisoned").clone();
        println!("request: {}", requests[0]);
        assert!(
            requests[0]["params"].is_array(),
            "`params` is an array: {}",
            requests[0]["params"]
        );
        assert_eq!(
            requests[0]["params"].as_array().map(Vec::len),
            Some(0),
            "and it is empty"
        );

        Ok(())
    }

    // ===================================================================================
    // RPC errors and HTTP errors are different things
    // ===================================================================================

    /// An RPC-level error arrives with **HTTP 200**, and must be reported as `RpcError` carrying aria2's own
    /// message.
    ///
    /// This is the distinction the plan asks for. A JSON-RPC error means **aria2 answered and refused** -- an
    /// unknown GID, an unauthorised token -- and the reason is aria2's, so it is worth surfacing. Reporting it as
    /// a transport failure would lose it.
    #[tokio::test]
    async fn a_json_rpc_error_object_becomes_an_rpc_error(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (port, _seen) = start_scripted_server(1, |_index, _request| {
            r#"{"jsonrpc":"2.0","id":"1","error":{"code":1,"message":"Unauthorized"}}"#.to_string()
        })
        .await?;

        let client = Aria2RpcClient::new(port, None);
        let result = client
            .call_method::<_, String>("aria2.tellStatus", "gid")
            .await;

        match &result {
            Err(Aria2Error::RpcError(message)) => {
                println!("rpc error message: {message}");
                assert!(
                    message.contains("Unauthorized"),
                    "aria2's own reason must reach the caller: {message}"
                );
            }
            other => panic!("expected an RpcError, got {other:?}"),
        }

        Ok(())
    }

    /// A transport failure is an `RpcError` with the transport's message, and **not** an `HttpError`.
    ///
    /// `HttpError` is reserved for a response that arrived with a non-success status; a connection that was never
    /// established has no status to report. This is the variant a caller sees when aria2 is not running, which is
    /// the common case.
    #[tokio::test]
    async fn a_connection_failure_is_an_rpc_error_and_not_an_http_error(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // A port with nothing listening: bind, read it, release it.
        let port = {
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let port = listener.local_addr()?.port();
            drop(listener);
            port
        };

        let client = Aria2RpcClient::new(port, None);
        let result = client
            .call_method::<_, String>("aria2.getVersion", ())
            .await;

        match &result {
            Err(Aria2Error::RpcError(message)) => {
                println!("connection failure message: {message}");
                assert!(!message.is_empty(), "the transport's message is kept");
            }
            Err(Aria2Error::HttpError { status, .. }) => {
                panic!("a connection that never opened has no HTTP status, but got {status}")
            }
            other => panic!("expected an RpcError, got {other:?}"),
        }

        // The two variants are distinguishable, which is what lets a caller treat "aria2 refused" differently
        // from "aria2 is unreachable".
        let http = Aria2Error::HttpError {
            status: reqwest::StatusCode::BAD_GATEWAY,
            body: "upstream".to_string(),
        };
        let rpc = Aria2Error::RpcError("refused".to_string());
        assert_ne!(
            std::mem::discriminant(&http),
            std::mem::discriminant(&rpc),
            "the variants a caller matches on must be distinct"
        );

        Ok(())
    }

    /// The two error kinds render differently, so a log line says which happened.
    #[test]
    fn the_two_error_kinds_are_told_apart_in_their_messages() {
        let http = Aria2Error::HttpError {
            status: reqwest::StatusCode::BAD_GATEWAY,
            body: "upstream unavailable".to_string(),
        };
        let rpc = Aria2Error::RpcError("Unauthorized".to_string());

        let http_text = http.to_string();
        let rpc_text = rpc.to_string();
        println!("http: {http_text}");
        println!("rpc:  {rpc_text}");

        assert!(
            http_text.contains("502"),
            "the status code is in the message: {http_text}"
        );
        assert!(
            http_text.contains("upstream unavailable"),
            "and the body, which is where a proxy explains itself: {http_text}"
        );
        assert!(rpc_text.contains("Unauthorized"));
        assert_ne!(http_text, rpc_text);
    }

    // ===================================================================================
    // The boundary: no timeout
    // ===================================================================================

    /// **Recorded, not fixed**: the client is built with `Client::new()` and **no timeout**, so a server that
    /// accepts the connection and never answers leaves the call pending.
    ///
    /// The plan lists 连接失败/超时. A connection failure is covered above; a **timeout is not covered, because
    /// there is no timeout to cover**. The only way to demonstrate the wait is to wait, and a test that waits
    /// either passes slowly or fails on a loaded machine.
    ///
    /// What this does instead is pin the thing that would have to change, so adding a timeout is a deliberate
    /// edit with a test around it rather than a silent behaviour change. The 500 ms bound is imposed **by the
    /// test**: if the client had a shorter timeout the call would resolve, and the test fails to say so.
    #[tokio::test]
    async fn the_client_is_built_without_a_timeout_which_is_recorded_rather_than_tested(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // A server that accepts and then does nothing at all.
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let held = tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                drop(stream);
            }
        });

        let client = Aria2RpcClient::new(port, None);
        let outcome = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            client.call_method::<_, String>("aria2.getVersion", ()),
        )
        .await;

        match outcome {
            Err(_elapsed) => {
                println!(
                    "the call was still pending after 500ms, which is the recorded absence of a client timeout"
                );
            }
            Ok(Ok(_)) => panic!("a server that never answered produced a result"),
            Ok(Err(e)) => {
                panic!("the call resolved with an error, so the client does time out: {e}")
            }
        }

        held.abort();
        Ok(())
    }
}
