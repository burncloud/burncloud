# process.rs

## 公有函数

### check_port_available
- 函数定义：`pub fn check_port_available(port: u16) -> bool`
- 入参：`port: u16`；待检测的本地 TCP 端口。
- 返回值：`bool`；能够绑定 `127.0.0.1:port` 时为 `true`。
- 错误信息：不返回错误；绑定失败转换为 `false`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[绑定 127.0.0.1 与端口]
  B --> C{绑定成功}
  C -->|是| D[返回 true]
  C -->|否| E[返回 false]
```

### find_available_port
- 函数定义：`pub fn find_available_port() -> Aria2Result<u16>`
- 入参：无。
- 返回值：`Aria2Result<u16>`；返回端口范围内第一个可用端口。
- 错误信息：没有可用端口时返回 `Aria2Error::PortError`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[遍历默认端口范围]
  B --> C[检查当前端口]
  C --> D{端口可用}
  D -->|是| E[返回 Ok 端口]
  D -->|否| F{还有候选端口}
  F -->|是| C
  F -->|否| G[返回 PortError]
```

### kill_existing_aria2
- 函数定义：`pub fn kill_existing_aria2()`
- 入参：无。
- 返回值：无（`()`）；尝试结束所有 `aria2c.exe` 进程。
- 错误信息：不返回错误；`taskkill` 执行失败被忽略。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[执行 taskkill /F /IM aria2c.exe]
  B --> C[忽略命令结果]
  C --> D[结束]
```

### start_aria2_rpc
- 函数定义：`pub async fn start_aria2_rpc(config: &Aria2Config) -> Aria2Result<Aria2Instance>`
- 入参：`config: &Aria2Config`；aria2 可执行文件、下载目录、连接数、分片大小及可选密钥的配置。
- 返回值：`Aria2Result<Aria2Instance>`；成功时返回子进程、实际端口和配置副本。
- 错误信息：端口不可用时返回 `PortError`；启动进程失败时返回 `ProcessError`；RPC 未就绪时返回 `RpcError`；清理旧进程失败被忽略。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[清理已有 aria2c.exe]
  B --> C[查找可用端口]
  C --> D{找到端口}
  D -->|否| X[返回 PortError]
  D -->|是| E[构建 aria2 启动命令]
  E --> F{配置密钥存在}
  F -->|是| G[添加 rpc-secret 参数]
  F -->|否| H[启动子进程]
  G --> H
  H --> I{启动成功}
  I -->|否| Y[返回 ProcessError]
  I -->|是| J[构造 Aria2Instance]
  J --> K[等待 RPC 就绪]
  K --> L{RPC 就绪}
  L -->|否| Z[返回 RpcError]
  L -->|是| M[返回实例]
```

## 私有函数

### wait_for_rpc_ready
- 函数定义：`async fn wait_for_rpc_ready(port: u16, secret: &Option<String>) -> Aria2Result<()>`
- 入参：`port` 为 RPC 端口；`secret` 为可选认证密钥。
- 返回值：`Aria2Result<()>`；在最多 30 次检测内获得成功响应时返回 `Ok(())`。
- 错误信息：所有检测未成功时返回 `Aria2Error::RpcError("RPC 服务启动超时")`；单次请求失败或非成功状态被忽略。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[创建 HTTP 客户端和 RPC 地址]
  B --> C[开始第 N 次检测]
  C --> D{密钥存在}
  D -->|是| E[添加 token 参数]
  D -->|否| F[构建 getVersion 请求]
  E --> F
  F --> G[发送请求]
  G --> H{收到成功 HTTP 状态}
  H -->|是| I[返回 Ok]
  H -->|否| J[等待 1 秒]
  J --> K{已检测 30 次}
  K -->|否| C
  K -->|是| L[返回 RpcError]
```
