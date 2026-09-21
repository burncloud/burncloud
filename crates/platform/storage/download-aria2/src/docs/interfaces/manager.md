# manager.rs 函数文档

## 公有函数

### new

- 函数定义：
  ```rust
  pub fn new() -> Self
  ```
- 入参：无。
- 返回值：`Self`，使用 `Aria2Config::default()` 创建的未启动管理器。
- 错误信息：无；不执行外部操作。

### with_config

- 函数定义：
  ```rust
  pub fn with_config(config: Aria2Config) -> Self
  ```
- 入参：
  - `config: Aria2Config`：管理器将使用的 aria2 配置。
- 返回值：`Self`，使用指定配置创建的未启动管理器。
- 错误信息：无；不执行外部操作。

### download_and_setup

- 函数定义：
  ```rust
  pub async fn download_and_setup(&mut self) -> Aria2Result<()>
  ```
- 入参：
  - `&mut self`：要更新二进制文件路径的管理器。
- 返回值：`Aria2Result<()>`。成功时将下载到的 `aria2c.exe` 路径写入内部配置。
- 错误信息：传播 `download_aria2` 返回的 `Aria2Error::DownloadError`。

### start_daemon

- 函数定义：
  ```rust
  pub async fn start_daemon(&mut self) -> Aria2Result<()>
  ```
- 入参：
  - `&mut self`：要创建并启动守护进程的管理器。
- 返回值：`Aria2Result<()>`。成功时在管理器中保存已启动的 `Aria2Daemon`。
- 错误信息：
  - 已持有守护进程时返回 `Aria2Error::DaemonError("守护进程已存在")`。
  - 启动失败时传播 `Aria2Daemon::start` 返回的错误。

### get_rpc_client

- 函数定义：
  ```rust
  pub fn get_rpc_client(&self) -> Option<&Aria2RpcClient>
  ```
- 入参：
  - `&self`：管理器对象。
- 返回值：`Option<&Aria2RpcClient>`；当前实现始终返回 `None`。
- 错误信息：无。

### create_rpc_client

- 函数定义：
  ```rust
  pub fn create_rpc_client(&self) -> Option<Aria2RpcClient>
  ```
- 入参：
  - `&self`：管理器对象。
- 返回值：`Option<Aria2RpcClient>`。守护进程存在且具有运行实例时，返回新建的独立 RPC 客户端；否则返回 `None`。
- 错误信息：不返回 `Result`；下层互斥锁中毒会被恢复处理。

### shutdown

- 函数定义：
  ```rust
  pub async fn shutdown(&mut self) -> Aria2Result<()>
  ```
- 入参：
  - `&mut self`：要关闭的管理器。
- 返回值：`Aria2Result<()>`。成功时停止已有守护进程并清除其引用。
- 错误信息：不主动返回错误；守护进程停止期间的进程终止错误会被忽略，因此当前实现始终返回 `Ok(())`。

### is_running

- 函数定义：
  ```rust
  pub fn is_running(&self) -> bool
  ```
- 入参：
  - `&self`：管理器对象。
- 返回值：`bool`。管理器持有守护进程且其运行标记为真时返回 `true`，否则返回 `false`。
- 错误信息：无。

### quick_start

- 函数定义：
  ```rust
  pub async fn quick_start() -> Aria2Result<Aria2Manager>
  ```
- 入参：无。
- 返回值：`Aria2Result<Aria2Manager>`。成功时返回已完成下载配置并启动守护进程的管理器。
- 错误信息：传播 `download_and_setup` 或 `start_daemon` 返回的 `Aria2Error`。

## 私有函数

### default

- 函数定义：
  ```rust
  fn default() -> Self
  ```
- 入参：无。
- 返回值：`Self`，等同于 `Aria2Manager::new()` 的默认管理器。
- 错误信息：无；不执行外部操作。
