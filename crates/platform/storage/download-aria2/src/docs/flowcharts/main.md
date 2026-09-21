# main.rs

## 公有函数
本文件未定义 `pub` 函数或方法。

## 私有函数

### main
- 函数定义：`async fn main() -> Aria2Result<()>`
- 入参：无。
- 返回值：`Aria2Result<()>`；成功时完成启动、测试、等待和关闭。
- 错误信息：传播 `quick_start`、`test_basic_operations`、`test_download` 或 `shutdown` 的错误。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[quick_start]
  B --> C{成功}
  C -->|否| X[传播 Aria2Error]
  C -->|是| D{可创建 RPC 客户端}
  D -->|是| E[执行基础操作测试]
  E --> F{成功}
  F -->|否| X
  F -->|是| G[执行下载测试]
  G --> H{成功}
  H -->|否| X
  H -->|是| I[等待 2 秒]
  D -->|否| I
  I --> J[关闭管理器]
  J --> K{成功}
  K -->|否| X
  K -->|是| L[返回 Ok]
```

### test_basic_operations
- 函数定义：`async fn test_basic_operations(client: &Aria2RpcClient) -> Aria2Result<()>`
- 入参：`client: &Aria2RpcClient`；已连接的 RPC 客户端。
- 返回值：`Aria2Result<()>`；当前实现始终返回 `Ok(())`。
- 错误信息：`get_global_stat` 和 `tell_active` 失败被忽略。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[查询全局统计]
  B --> C{查询成功}
  C -->|是| D[输出统计]
  C -->|否| E[忽略错误]
  D --> F[查询活跃任务]
  E --> F
  F --> G{查询成功}
  G -->|是| H[输出数量]
  G -->|否| I[忽略错误]
  H --> J[返回 Ok]
  I --> J
```

### test_download
- 函数定义：`async fn test_download(client: &Aria2RpcClient) -> Aria2Result<()>`
- 入参：`client: &Aria2RpcClient`；已连接的 RPC 客户端。
- 返回值：`Aria2Result<()>`；当前实现始终返回 `Ok(())`。
- 错误信息：添加任务或状态查询失败不会返回，仅输出或忽略。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[构造测试 URL 和下载配置]
  B --> C[调用 add_uri]
  C --> D{添加成功}
  D -->|否| E[输出错误]
  D -->|是| F[输出 GID 并等待 1 秒]
  F --> G[查询任务状态]
  G --> H{查询成功}
  H -->|是| I[输出状态]
  H -->|否| J[忽略错误]
  E --> K[返回 Ok]
  I --> K
  J --> K
```
