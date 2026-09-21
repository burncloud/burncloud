# main.rs 函数文档

## 公有函数

本文件未定义 `pub` 函数或方法。

## 私有函数

### main

- 函数定义：
  ```rust
  async fn main() -> Aria2Result<()>
  ```
- 入参：无。
- 返回值：`Aria2Result<()>`。成功时完成管理器启动、基础操作测试、下载测试、等待和关闭流程。
- 错误信息：`quick_start`、`test_basic_operations`、`test_download` 或 `manager.shutdown` 返回的 `Aria2Error` 会被传播。

### test_basic_operations

- 函数定义：
  ```rust
  async fn test_basic_operations(client: &Aria2RpcClient) -> Aria2Result<()>
  ```
- 入参：
  - `client: &Aria2RpcClient`：已连接的 aria2 RPC 客户端。
- 返回值：`Aria2Result<()>`。成功时完成全局统计和活跃任务查询的测试输出。
- 错误信息：不主动返回错误；`get_global_stat` 和 `tell_active` 的错误会被忽略，因此当前实现始终返回 `Ok(())`。

### test_download

- 函数定义：
  ```rust
  async fn test_download(client: &Aria2RpcClient) -> Aria2Result<()>
  ```
- 入参：
  - `client: &Aria2RpcClient`：已连接的 aria2 RPC 客户端。
- 返回值：`Aria2Result<()>`。成功时提交测试下载任务，并在成功添加后查询一次任务状态。
- 错误信息：不主动返回错误；`add_uri` 和 `tell_status` 的错误会被记录到标准输出或忽略，因此当前实现始终返回 `Ok(())`。
