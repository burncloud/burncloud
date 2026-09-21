# monitor.rs 函数文档

## 公有函数

无。

## 私有函数

### start_progress_monitor

- 函数定义：
  ```rust
  async fn start_progress_monitor(&self, gid: &str)
  ```
- 入参：`gid: &str`，要监控的下载任务 GID；`&self` 提供 aria2 管理器和数据库引用。
- 返回值：无（`()`）。创建后台 Tokio 任务，定期同步任务状态和进度。
- 错误信息：不返回错误；RPC 客户端不可用时结束监控，状态查询失败时继续循环，数据库更新失败被忽略，数值解析失败使用 `0`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[克隆 aria2、数据库引用和 GID]
  B --> C[创建后台 Tokio 任务]
  C --> D{RPC 客户端可用}
  D -->|否| E[结束监控]
  D -->|是| F[查询任务状态]
  F --> G{查询成功}
  G -->|否| H[等待 2 秒]
  G -->|是| I[更新数据库状态]
  I --> J[解析总大小、完成量和速度]
  J --> K[更新数据库进度]
  K --> L{状态为 complete 或 error}
  L -->|是| E
  L -->|否| H
  H --> D
```

### restore_incomplete_downloads

- 函数定义：
  ```rust
  async fn restore_incomplete_downloads(&self) -> Result<Vec<String>>
  ```
- 入参：`&self`，下载管理器。
- 返回值：`Result<Vec<String>>`；返回成功重新提交并开始监控的任务 GID 列表。
- 错误信息：RPC 客户端不存在或数据库查询失败时返回错误；URI JSON 解析失败使用空列表；单个任务重新提交或 GID 更新失败被忽略并继续处理。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[创建 RPC 客户端]
  B --> C{客户端存在}
  C -->|否| X[返回客户端未就绪 Aria2 错误]
  C -->|是| D[查询数据库 active 任务]
  D --> E{查询成功}
  E -->|否| Y[返回数据库错误]
  E -->|是| F[初始化 restored 列表]
  F --> G[遍历未完成任务]
  G --> H[解析 URI JSON]
  H --> I{URI 列表非空}
  I -->|否| J{还有任务}
  I -->|是| K[构造下载配置]
  K --> L[调用 add_uri]
  L --> M{重新提交成功}
  M -->|否| J
  M -->|是| N[尝试更新数据库 GID]
  N --> O[启动进度监控并加入 restored]
  O --> J
  J -->|是| G
  J -->|否| P[返回 restored 列表]
```
