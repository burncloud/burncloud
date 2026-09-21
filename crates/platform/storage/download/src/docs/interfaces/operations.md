# operations.rs 函数文档

## 公有函数

### new

- 函数定义：
  ```rust
  pub async fn new() -> Result<Self>
  ```
- 入参：无。
- 返回值：`Result<Self>`。成功时返回已启动 aria2、已初始化数据库并完成未完成下载恢复的 `DownloadManager`。
- 错误信息：
  - `quick_start` 失败时，通过 `From` 转换返回 `DownloadError::Aria2`。
  - `DownloadDB::new` 或 `restore_incomplete_downloads` 失败时，通过 `From` 转换返回 `DownloadError::Database` 或 `DownloadError::Aria2`。

### add_download

- 函数定义：
  ```rust
  pub async fn add_download(&self, url: &str, download_dir: Option<&str>) -> Result<String>
  ```
- 入参：
  - `&self`：已初始化的下载管理器。
  - `url: &str`：待下载资源的 URL。
  - `download_dir: Option<&str>`：可选下载目录；为 `None` 时使用 `./downloads`。
- 返回值：`Result<String>`。成功时返回 aria2 下载任务的 GID，并已将任务写入数据库和启动进度监控。
- 错误信息：
  - 无可用 RPC 客户端时返回 `DownloadError::Aria2(Aria2Error::RpcError("客户端未就绪"))`。
  - 提交 aria2 下载任务失败时返回 `DownloadError::Aria2`。
  - 向数据库添加任务失败时返回 `DownloadError::Database`。
  - 文件名提取失败不返回错误，`DownloadOptions.out` 为 `None`。

### get_status

- 函数定义：
  ```rust
  pub async fn get_status(&self, gid: &str) -> Result<burncloud_download_aria2::DownloadStatus>
  ```
- 入参：
  - `&self`：已初始化的下载管理器。
  - `gid: &str`：要查询的 aria2 下载任务 GID。
- 返回值：`Result<burncloud_download_aria2::DownloadStatus>`。成功时返回任务状态，并将状态、总大小、已完成大小和下载速度同步到数据库。
- 错误信息：
  - 无可用 RPC 客户端时返回 `DownloadError::Aria2(Aria2Error::RpcError("客户端未就绪"))`。
  - 查询 aria2 状态失败时返回 `DownloadError::Aria2`。
  - 更新数据库状态或进度失败时返回 `DownloadError::Database`。
  - 三个数值字段解析失败时使用 `0`，不返回错误。

### pause

- 函数定义：
  ```rust
  pub async fn pause(&self, gid: &str) -> Result<()>
  ```
- 入参：
  - `&self`：已初始化的下载管理器。
  - `gid: &str`：要暂停的 aria2 下载任务 GID。
- 返回值：`Result<()>`。成功时暂停 aria2 任务，并将数据库中的状态更新为 `paused`。
- 错误信息：
  - 无可用 RPC 客户端时返回 `DownloadError::Aria2(Aria2Error::RpcError("客户端未就绪"))`。
  - 暂停 aria2 任务失败时返回 `DownloadError::Aria2`。
  - 更新数据库状态失败时返回 `DownloadError::Database`。

### resume

- 函数定义：
  ```rust
  pub async fn resume(&self, gid: &str) -> Result<()>
  ```
- 入参：
  - `&self`：已初始化的下载管理器。
  - `gid: &str`：要恢复的 aria2 下载任务 GID。
- 返回值：`Result<()>`。成功时恢复 aria2 任务，并将数据库中的状态更新为 `active`。
- 错误信息：
  - 无可用 RPC 客户端时返回 `DownloadError::Aria2(Aria2Error::RpcError("客户端未就绪"))`。
  - 恢复 aria2 任务失败时返回 `DownloadError::Aria2`。
  - 更新数据库状态失败时返回 `DownloadError::Database`。

### remove

- 函数定义：
  ```rust
  pub async fn remove(&self, gid: &str) -> Result<()>
  ```
- 入参：
  - `&self`：已初始化的下载管理器。
  - `gid: &str`：要移除的 aria2 下载任务 GID。
- 返回值：`Result<()>`。成功时移除 aria2 任务并删除对应数据库记录。
- 错误信息：
  - 无可用 RPC 客户端时返回 `DownloadError::Aria2(Aria2Error::RpcError("客户端未就绪"))`。
  - 移除 aria2 任务失败时返回 `DownloadError::Aria2`。
  - 删除数据库记录失败时返回 `DownloadError::Database`。

## 私有函数

无。
