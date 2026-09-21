# process.rs 函数文档

## 公有函数

### check_port_available

- 函数定义：
  ```rust
  pub fn check_port_available(port: u16) -> bool
  ```
- 入参：
  - `port: u16`：待检测的本地 TCP 端口号。
- 返回值：`bool`。能够绑定到 `127.0.0.1:port` 时返回 `true`，否则返回 `false`。
- 错误信息：不返回 `Result`；端口绑定失败会转换为 `false`。

### find_available_port

- 函数定义：
  ```rust
  pub fn find_available_port() -> Aria2Result<u16>
  ```
- 入参：无。
- 返回值：`Aria2Result<u16>`。成功时返回 `[DEFAULT_PORT, DEFAULT_PORT + MAX_PORT_RANGE]` 中第一个可绑定端口。
- 错误信息：范围内无可用端口时返回 `Aria2Error::PortError("未找到可用端口")`。

### kill_existing_aria2

- 函数定义：
  ```rust
  pub fn kill_existing_aria2()
  ```
- 入参：无。
- 返回值：无（`()`）；尝试执行 `taskkill /F /IM aria2c.exe`。
- 错误信息：不返回错误；命令执行或读取输出失败会被忽略。

### start_aria2_rpc

- 函数定义：
  ```rust
  pub async fn start_aria2_rpc(config: &Aria2Config) -> Aria2Result<Aria2Instance>
  ```
- 入参：
  - `config: &Aria2Config`：aria2 可执行文件、端口、下载目录、连接数、分片大小及可选密钥的配置。
- 返回值：`Aria2Result<Aria2Instance>`。成功时返回包含子进程、实际端口和配置副本的运行实例。
- 错误信息：
  - 没有可用端口时传播 `Aria2Error::PortError`。
  - 创建 aria2 子进程失败时返回 `Aria2Error::ProcessError`。
  - RPC 服务在轮询期内未就绪时传播 `Aria2Error::RpcError`。
  - 清理已有 aria2 进程的失败会被忽略。

## 私有函数

### wait_for_rpc_ready

- 函数定义：
  ```rust
  async fn wait_for_rpc_ready(port: u16, secret: &Option<String>) -> Aria2Result<()>
  ```
- 入参：
  - `port: u16`：待检测的 aria2 JSON-RPC 端口。
  - `secret: &Option<String>`：可选 RPC 密钥；存在时作为 `token:<secret>` 参数发送。
- 返回值：`Aria2Result<()>`。在最多 30 次轮询中收到成功 HTTP 状态时返回 `Ok(())`。
- 错误信息：30 次轮询后仍未收到成功响应时返回 `Aria2Error::RpcError("RPC 服务启动超时")`；单次请求错误或非成功状态会被忽略并继续轮询。
