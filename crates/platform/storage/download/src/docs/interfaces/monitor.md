# monitor.rs 函数文档

## 公有函数

无。

## 私有函数

### start_progress_monitor

- 函数定义：
  ```rust
  async fn start_progress_monitor(&self, gid: &str)
  ```
- 入参：
  - `&self`：下载管理器；其 aria2 管理器和数据库引用会被克隆到后台任务。
  - `gid: &str`：要监控的 aria2 下载任务 GID。
- 返回值：无（`()`）。创建后台 Tokio 任务，每两秒查询一次任务状态，并尝试同步数据库状态和进度；状态为 `complete` 或 `error` 时结束监控。
- 错误信息：不返回错误；获取 RPC 客户端失败时停止监控，状态查询失败时继续下一轮，数据库状态和进度更新失败均被忽略，数值字段解析失败时使用 `0`。

### restore_incomplete_downloads

- 函数定义：
  ```rust
  async fn restore_incomplete_downloads(&self) -> Result<Vec<String>>
  ```
- 入参：
  - `&self`：下载管理器。
- 返回值：`Result<Vec<String>>`。成功时返回已重新创建并开始监控的下载任务 GID 列表；无可恢复任务时返回空列表。
- 错误信息：
  - 无可用 RPC 客户端时返回 `DownloadError::Aria2(Aria2Error::RpcError("客户端未就绪"))`。
  - 查询数据库中的 `active` 任务失败时返回 `DownloadError::Database`。
  - 已保存 URI 的 JSON 解析失败时使用空列表，不返回错误。
  - 单个任务重新提交 aria2 失败、更新 GID 失败时会被忽略，函数继续恢复其他任务。
