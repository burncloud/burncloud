# daemon.rs

## 公有函数

### new
- 函数定义：`pub fn new(config: Aria2Config) -> Self`
- 入参：`config: Aria2Config`；aria2 运行配置。
- 返回值：`Self`；未启动的 `Aria2Daemon`。
- 错误信息：无。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[初始化空实例槽位]
  B --> C[保存配置]
  C --> D[初始化运行标记为 false]
  D --> E[返回守护进程]
```

### start
- 函数定义：`pub async fn start(&mut self) -> Aria2Result<()>`
- 入参：`&mut self`；待启动的守护进程。
- 返回值：`Aria2Result<()>`；成功时服务启动并创建监控任务。
- 错误信息：重复启动时返回 `DaemonError`；首次启动失败时传播端口、进程或 RPC 错误；监控中的重启失败被忽略。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B{运行标记为 true}
  B -->|是| X[返回 DaemonError]
  B -->|否| C[启动 aria2 RPC]
  C --> D{启动成功}
  D -->|否| Y[传播 Aria2Error]
  D -->|是| E[保存运行实例并置运行标记]
  E --> F[创建异步监控任务]
  F --> G[返回 Ok]
  H[监控循环] --> I[每秒检查实例状态]
  I --> J{需要重启}
  J -->|否| H
  J -->|是| K[尝试启动新实例]
  K --> L{启动成功}
  L -->|是| M[替换运行实例]
  L -->|否| H
  M --> H
```

### stop
- 函数定义：`pub async fn stop(&mut self)`
- 入参：`&mut self`；待停止的守护进程。
- 返回值：无（`()`）；运行标记被清除，保存的实例被移除。
- 错误信息：不返回错误；终止子进程失败被忽略。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[运行标记设为 false]
  B --> C{存在运行实例}
  C -->|是| D[调用 instance.kill 并忽略结果]
  C -->|否| E[清空实例槽位]
  D --> E
  E --> F[结束]
```

### get_rpc_client
- 函数定义：`pub fn get_rpc_client(&self) -> Option<Aria2RpcClient>`
- 入参：`&self`；守护进程对象。
- 返回值：`Option<Aria2RpcClient>`；实例存在时返回新客户端，否则为 `None`。
- 错误信息：不返回错误；互斥锁中毒时恢复其值。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[锁定实例槽位]
  B --> C{实例存在}
  C -->|是| D[用端口和密钥创建 RPC 客户端]
  C -->|否| E[返回 None]
  D --> F[返回 Some 客户端]
```

### is_running
- 函数定义：`pub fn is_running(&self) -> bool`
- 入参：`&self`；守护进程对象。
- 返回值：`bool`；原子运行标记的当前值。
- 错误信息：无。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[读取原子运行标记]
  B --> C[返回 bool]
```
