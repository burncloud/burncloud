# rpc.rs 函数文档

## 公有函数

### new

- 函数定义：
  ```rust
  pub fn new(port: u16, secret: Option<String>) -> Self
  ```
- 入参：
  - `port: u16`：aria2 JSON-RPC 服务监听端口。
  - `secret: Option<String>`：可选 RPC 密钥。
- 返回值：`Self`，包含 HTTP 客户端、`http://localhost:<port>/jsonrpc` 地址、密钥和初始请求编号的客户端。
- 错误信息：无；不执行网络请求。

### add_uri

- 函数定义：
  ```rust
  pub async fn add_uri(
      &self,
      uris: Vec<String>,
      options: Option<DownloadOptions>,
  ) -> Aria2Result<String>
  ```
- 入参：
  - `&self`：RPC 客户端。
  - `uris: Vec<String>`：待下载的 URI 列表。
  - `options: Option<DownloadOptions>`：可选下载配置；存在时与 URI 一同传给 `aria2.addUri`。
- 返回值：`Aria2Result<String>`。发现相同任务时返回其 GID；否则返回新建任务的 GID。
- 错误信息：重复任务检测或 `aria2.addUri` 调用失败时传播 `Aria2Error::RpcError`。

### tell_status

- 函数定义：
  ```rust
  pub async fn tell_status(&self, gid: &str) -> Aria2Result<DownloadStatus>
  ```
- 入参：
  - `&self`：RPC 客户端。
  - `gid: &str`：下载任务的 GID。
- 返回值：`Aria2Result<DownloadStatus>`，指定任务的状态。
- 错误信息：`aria2.tellStatus` 的序列化、请求、响应解析或服务端错误会返回 `Aria2Error::RpcError`。

### tell_active

- 函数定义：
  ```rust
  pub async fn tell_active(&self) -> Aria2Result<Vec<DownloadStatus>>
  ```
- 入参：
  - `&self`：RPC 客户端。
- 返回值：`Aria2Result<Vec<DownloadStatus>>`，全部活跃任务的状态列表。
- 错误信息：`aria2.tellActive` 的序列化、请求、响应解析或服务端错误会返回 `Aria2Error::RpcError`。

### tell_waiting

- 函数定义：
  ```rust
  pub async fn tell_waiting(&self, offset: u32, num: u32) -> Aria2Result<Vec<DownloadStatus>>
  ```
- 入参：
  - `&self`：RPC 客户端。
  - `offset: u32`：等待任务列表的起始偏移量。
  - `num: u32`：要返回的任务数量。
- 返回值：`Aria2Result<Vec<DownloadStatus>>`，等待任务的状态列表。
- 错误信息：`aria2.tellWaiting` 的序列化、请求、响应解析或服务端错误会返回 `Aria2Error::RpcError`。

### tell_stopped

- 函数定义：
  ```rust
  pub async fn tell_stopped(&self, offset: u32, num: u32) -> Aria2Result<Vec<DownloadStatus>>
  ```
- 入参：
  - `&self`：RPC 客户端。
  - `offset: u32`：已停止任务列表的起始偏移量。
  - `num: u32`：要返回的任务数量。
- 返回值：`Aria2Result<Vec<DownloadStatus>>`，已停止任务的状态列表。
- 错误信息：`aria2.tellStopped` 的序列化、请求、响应解析或服务端错误会返回 `Aria2Error::RpcError`。

### get_files

- 函数定义：
  ```rust
  pub async fn get_files(&self, gid: &str) -> Aria2Result<Vec<FileInfo>>
  ```
- 入参：
  - `&self`：RPC 客户端。
  - `gid: &str`：下载任务的 GID。
- 返回值：`Aria2Result<Vec<FileInfo>>`，指定任务关联的文件信息。
- 错误信息：`aria2.getFiles` 的序列化、请求、响应解析或服务端错误会返回 `Aria2Error::RpcError`。

### get_global_stat

- 函数定义：
  ```rust
  pub async fn get_global_stat(&self) -> Aria2Result<GlobalStat>
  ```
- 入参：
  - `&self`：RPC 客户端。
- 返回值：`Aria2Result<GlobalStat>`，全局下载统计信息。
- 错误信息：`aria2.getGlobalStat` 的序列化、请求、响应解析或服务端错误会返回 `Aria2Error::RpcError`。

### pause

- 函数定义：
  ```rust
  pub async fn pause(&self, gid: &str) -> Aria2Result<String>
  ```
- 入参：
  - `&self`：RPC 客户端。
  - `gid: &str`：要暂停的下载任务 GID。
- 返回值：`Aria2Result<String>`，aria2 返回的任务 GID。
- 错误信息：`aria2.pause` 的序列化、请求、响应解析或服务端错误会返回 `Aria2Error::RpcError`。

### unpause

- 函数定义：
  ```rust
  pub async fn unpause(&self, gid: &str) -> Aria2Result<String>
  ```
- 入参：
  - `&self`：RPC 客户端。
  - `gid: &str`：要恢复的下载任务 GID。
- 返回值：`Aria2Result<String>`，aria2 返回的任务 GID。
- 错误信息：`aria2.unpause` 的序列化、请求、响应解析或服务端错误会返回 `Aria2Error::RpcError`。

### remove

- 函数定义：
  ```rust
  pub async fn remove(&self, gid: &str) -> Aria2Result<String>
  ```
- 入参：
  - `&self`：RPC 客户端。
  - `gid: &str`：要移除的下载任务 GID。
- 返回值：`Aria2Result<String>`，aria2 返回的任务 GID。
- 错误信息：`aria2.remove` 的序列化、请求、响应解析或服务端错误会返回 `Aria2Error::RpcError`。

### shutdown

- 函数定义：
  ```rust
  pub async fn shutdown(&self) -> Aria2Result<String>
  ```
- 入参：
  - `&self`：RPC 客户端。
- 返回值：`Aria2Result<String>`，aria2 返回的关闭操作结果。
- 错误信息：`aria2.shutdown` 的序列化、请求、响应解析或服务端错误会返回 `Aria2Error::RpcError`。

## 私有函数

### call_method

- 函数定义：
  ```rust
  async fn call_method<T, R>(&self, method: &str, params: T) -> Aria2Result<R>
  where
      T: Serialize,
      R: for<'de> Deserialize<'de>,
  ```
- 入参：
  - `&self`：RPC 客户端；其密钥会在存在时添加为第一个 JSON-RPC 参数。
  - `method: &str`：aria2 JSON-RPC 方法名。
  - `params: T`：实现 `Serialize` 的 RPC 参数；序列化为数组时会展开为多个参数，`null` 不会加入参数列表。
- 返回值：`Aria2Result<R>`。成功时将 JSON-RPC 的 `result` 字段反序列化为满足 `for<'de> Deserialize<'de>` 的类型 `R`。
- 错误信息：参数序列化、HTTP 请求、响应 JSON 解析、服务端 `error` 字段存在或结果反序列化失败时返回 `Aria2Error::RpcError`。

### find_existing_task

- 函数定义：
  ```rust
  async fn find_existing_task(
      &self,
      uris: &[String],
      options: &Option<DownloadOptions>,
  ) -> Aria2Result<Option<String>>
  ```
- 入参：
  - `&self`：RPC 客户端。
  - `uris: &[String]`：待匹配的 URI 列表。
  - `options: &Option<DownloadOptions>`：用于比较目标下载目录的可选配置。
- 返回值：`Aria2Result<Option<String>>`。匹配到任务时返回 `Ok(Some(gid))`，否则返回 `Ok(None)`。
- 错误信息：活跃、等待、已停止任务列表查询及单个状态查询失败会被忽略；`is_same_task` 返回错误时会被传播。

### is_same_task

- 函数定义：
  ```rust
  async fn is_same_task(
      &self,
      status: &DownloadStatus,
      uris: &[String],
      options: &Option<DownloadOptions>,
  ) -> Aria2Result<bool>
  ```
- 入参：
  - `&self`：RPC 客户端。
  - `status: &DownloadStatus`：待比较的现有任务状态，使用其 `gid` 查询文件信息。
  - `uris: &[String]`：待比较的 URI 列表。
  - `options: &Option<DownloadOptions>`：可选下载配置；指定 `dir` 时还会比较文件路径前缀。
- 返回值：`Aria2Result<bool>`。存在相同 URI 且目录匹配的任务时返回 `Ok(true)`；未指定目录时 URI 相同即返回 `Ok(true)`；其他情况返回 `Ok(false)`。
- 错误信息：`get_files` 查询失败会被忽略并返回 `Ok(false)`；当前函数体不主动构造或传播错误。
