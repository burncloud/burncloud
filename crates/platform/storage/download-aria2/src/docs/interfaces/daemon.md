# daemon.rs 函数文档

## 公有函数

### new

- 函数定义：
  ```rust
  pub fn new(config: Aria2Config) -> Self
  ```
- 入参：
  - `config: Aria2Config`：aria2 运行配置，保存到新建的守护进程对象。
- 返回值：`Self`，未启动的 `Aria2Daemon`。
- 错误信息：无；不执行外部操作。

### start

- 函数定义：
  ```rust
  pub async fn start(&mut self) -> Aria2Result<()>
  ```
- 入参：
  - `&mut self`：要启动的守护进程对象。
- 返回值：`Aria2Result<()>`。成功时保存运行实例、设置运行标记并启动监控任务。
- 错误信息：
  - 已处于运行状态时返回 `Aria2Error::DaemonError("守护进程已在运行")`。
  - 初次启动 aria2 RPC 服务时，传播 `start_aria2_rpc` 返回的 `PortError`、`ProcessError` 或 `RpcError`。
  - 监控任务中的后续重启失败会被忽略，不会通过本次调用返回。

### stop

- 函数定义：
  ```rust
  pub async fn stop(&mut self)
  ```
- 入参：
  - `&mut self`：要停止的守护进程对象。
- 返回值：无（`()`）；清除运行标记和保存的运行实例。
- 错误信息：不返回错误；调用 `Aria2Instance::kill` 产生的错误会被忽略。

### get_rpc_client

- 函数定义：
  ```rust
  pub fn get_rpc_client(&self) -> Option<Aria2RpcClient>
  ```
- 入参：
  - `&self`：守护进程对象。
- 返回值：`Option<Aria2RpcClient>`。当前实例存在时，返回使用实例端口和配置密钥创建的客户端；否则返回 `None`。
- 错误信息：不返回 `Result`；互斥锁中毒时恢复其中的值继续执行。

### is_running

- 函数定义：
  ```rust
  pub fn is_running(&self) -> bool
  ```
- 入参：
  - `&self`：守护进程对象。
- 返回值：`bool`，原子运行标记的当前值。
- 错误信息：无。
