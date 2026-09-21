# rpc.rs

## 公有函数

### new
- 函数定义：`pub fn new(port: u16, secret: Option<String>) -> Self`
- 入参：`port` 为 RPC 端口；`secret` 为可选 RPC 密钥。
- 返回值：`Self`；包含 HTTP 客户端、服务地址、密钥和请求编号的客户端。
- 错误信息：无；不执行网络请求。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[创建 HTTP Client]
  B --> C[构造 JSON-RPC 地址]
  C --> D[保存密钥并初始化请求编号]
  D --> E[返回客户端]
```

### add_uri
- 函数定义：
  ```rust
  pub async fn add_uri(
      &self,
      uris: Vec<String>,
      options: Option<DownloadOptions>,
  ) -> Aria2Result<String>
  ```
- 入参：`&self` 为 RPC 客户端；`uris` 为待下载 URI；`options` 为可选下载配置。
- 返回值：`Aria2Result<String>`；返回已有任务或新建任务的 GID。
- 错误信息：重复任务检测或 RPC 调用失败时返回 `RpcError`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[查找相同任务]
  B --> C{查找成功}
  C -->|否| X[返回 RpcError]
  C -->|是| D{存在相同任务}
  D -->|是| E[返回已有 GID]
  D -->|否| F{options 存在}
  F -->|是| G[调用 addUri，参数为 uris 和 options]
  F -->|否| H[调用 addUri，参数为 uris]
  G --> I[返回 RPC 结果]
  H --> I
```

### tell_status
- 函数定义：`pub async fn tell_status(&self, gid: &str) -> Aria2Result<DownloadStatus>`
- 入参：`&self` 为 RPC 客户端；`gid` 为任务标识。
- 返回值：`Aria2Result<DownloadStatus>`；指定任务状态。
- 错误信息：序列化、请求、响应解析或服务端错误时返回 `RpcError`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[call_method aria2.tellStatus]
  B --> C[返回 DownloadStatus 或 RpcError]
```

### tell_active
- 函数定义：`pub async fn tell_active(&self) -> Aria2Result<Vec<DownloadStatus>>`
- 入参：`&self`；RPC 客户端。
- 返回值：`Aria2Result<Vec<DownloadStatus>>`；活跃任务列表。
- 错误信息：序列化、请求、响应解析或服务端错误时返回 `RpcError`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[call_method aria2.tellActive]
  B --> C[返回状态列表或 RpcError]
```

### tell_waiting
- 函数定义：`pub async fn tell_waiting(&self, offset: u32, num: u32) -> Aria2Result<Vec<DownloadStatus>>`
- 入参：`&self` 为 RPC 客户端；`offset` 为起始偏移；`num` 为返回数量。
- 返回值：`Aria2Result<Vec<DownloadStatus>>`；等待任务列表。
- 错误信息：序列化、请求、响应解析或服务端错误时返回 `RpcError`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[call_method aria2.tellWaiting]
  B --> C[返回状态列表或 RpcError]
```

### tell_stopped
- 函数定义：`pub async fn tell_stopped(&self, offset: u32, num: u32) -> Aria2Result<Vec<DownloadStatus>>`
- 入参：`&self` 为 RPC 客户端；`offset` 为起始偏移；`num` 为返回数量。
- 返回值：`Aria2Result<Vec<DownloadStatus>>`；已停止任务列表。
- 错误信息：序列化、请求、响应解析或服务端错误时返回 `RpcError`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[call_method aria2.tellStopped]
  B --> C[返回状态列表或 RpcError]
```

### get_files
- 函数定义：`pub async fn get_files(&self, gid: &str) -> Aria2Result<Vec<FileInfo>>`
- 入参：`&self` 为 RPC 客户端；`gid` 为任务标识。
- 返回值：`Aria2Result<Vec<FileInfo>>`；任务关联的文件信息。
- 错误信息：序列化、请求、响应解析或服务端错误时返回 `RpcError`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[call_method aria2.getFiles]
  B --> C[返回文件列表或 RpcError]
```

### get_global_stat
- 函数定义：`pub async fn get_global_stat(&self) -> Aria2Result<GlobalStat>`
- 入参：`&self`；RPC 客户端。
- 返回值：`Aria2Result<GlobalStat>`；全局下载统计。
- 错误信息：序列化、请求、响应解析或服务端错误时返回 `RpcError`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[call_method aria2.getGlobalStat]
  B --> C[返回统计或 RpcError]
```

### pause
- 函数定义：`pub async fn pause(&self, gid: &str) -> Aria2Result<String>`
- 入参：`&self` 为 RPC 客户端；`gid` 为要暂停的任务标识。
- 返回值：`Aria2Result<String>`；aria2 返回的任务 GID。
- 错误信息：序列化、请求、响应解析或服务端错误时返回 `RpcError`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[call_method aria2.pause]
  B --> C[返回 GID 或 RpcError]
```

### unpause
- 函数定义：`pub async fn unpause(&self, gid: &str) -> Aria2Result<String>`
- 入参：`&self` 为 RPC 客户端；`gid` 为要恢复的任务标识。
- 返回值：`Aria2Result<String>`；aria2 返回的任务 GID。
- 错误信息：序列化、请求、响应解析或服务端错误时返回 `RpcError`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[call_method aria2.unpause]
  B --> C[返回 GID 或 RpcError]
```

### remove
- 函数定义：`pub async fn remove(&self, gid: &str) -> Aria2Result<String>`
- 入参：`&self` 为 RPC 客户端；`gid` 为要移除的任务标识。
- 返回值：`Aria2Result<String>`；aria2 返回的任务 GID。
- 错误信息：序列化、请求、响应解析或服务端错误时返回 `RpcError`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[call_method aria2.remove]
  B --> C[返回 GID 或 RpcError]
```

### shutdown
- 函数定义：`pub async fn shutdown(&self) -> Aria2Result<String>`
- 入参：`&self`；RPC 客户端。
- 返回值：`Aria2Result<String>`；aria2 返回的关闭结果。
- 错误信息：序列化、请求、响应解析或服务端错误时返回 `RpcError`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[call_method aria2.shutdown]
  B --> C[返回结果或 RpcError]
```

## 私有函数

### call_method
- 函数定义：
  ```rust
  async fn call_method<T, R>(&self, method: &str, params: T) -> Aria2Result<R>
  where
      T: Serialize,
      R: for<'de> Deserialize<'de>,
  ```
- 入参：`&self` 为 RPC 客户端；`method` 为 RPC 方法名；`params` 为可序列化参数。
- 返回值：`Aria2Result<R>`；将响应的 `result` 字段反序列化为 `R`。
- 错误信息：参数序列化、HTTP 请求、响应 JSON 解析、服务端返回 `error` 或结果反序列化失败时返回 `RpcError`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[初始化 RPC 参数]
  B --> C{配置密钥存在}
  C -->|是| D[添加 token 参数]
  C -->|否| E[序列化 params]
  D --> E
  E --> F{序列化成功}
  F -->|否| X[返回 RpcError]
  F -->|是| G{参数为数组}
  G -->|是| H[展开并加入参数]
  G -->|否| I{参数非 null}
  I -->|是| J[加入单个参数]
  I -->|否| K[生成请求编号和请求体]
  H --> K
  J --> K
  K --> L[发送 HTTP 请求]
  L --> M{请求成功且响应可解析}
  M -->|否| X
  M -->|是| N{响应含 error}
  N -->|是| X
  N -->|否| O[反序列化 result]
  O --> P{反序列化成功}
  P -->|否| X
  P -->|是| Q[返回结果]
```

### find_existing_task
- 函数定义：
  ```rust
  async fn find_existing_task(
      &self,
      uris: &[String],
      options: &Option<DownloadOptions>,
  ) -> Aria2Result<Option<String>>
  ```
- 入参：`&self` 为 RPC 客户端；`uris` 为目标 URI；`options` 为可选下载配置。
- 返回值：`Aria2Result<Option<String>>`；匹配时返回 GID，不匹配时为 `None`。
- 错误信息：三个任务列表查询和单个状态查询失败被忽略；`is_same_task` 的错误会被传播。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[查询活跃任务并在成功时加入列表]
  B --> C[查询等待任务并在成功时加入列表]
  C --> D[查询已停止任务并在成功时加入列表]
  D --> E[遍历全部任务]
  E --> F[查询当前任务状态]
  F --> G{状态查询成功}
  G -->|否| H{还有任务}
  G -->|是| I[调用 is_same_task]
  I --> J{调用成功且相同}
  J -->|调用失败| X[传播 RpcError]
  J -->|是| K[返回 Some GID]
  J -->|否| H
  H -->|是| E
  H -->|否| L[返回 Ok None]
```

### is_same_task
- 函数定义：
  ```rust
  async fn is_same_task(
      &self,
      status: &DownloadStatus,
      uris: &[String],
      options: &Option<DownloadOptions>,
  ) -> Aria2Result<bool>
  ```
- 入参：`&self` 为 RPC 客户端；`status` 提供待查任务 GID；`uris` 为目标 URI；`options` 提供可选目标目录。
- 返回值：`Aria2Result<bool>`；URI 和目录条件匹配时为 `true`，否则为 `false`。
- 错误信息：`get_files` 失败被忽略并返回 `Ok(false)`；当前实现不主动构造错误。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[通过 status.gid 查询文件]
  B --> C{查询成功}
  C -->|否| X[返回 Ok false]
  C -->|是| D[遍历文件和目标 URI]
  D --> E{URI 匹配}
  E -->|否| F{还有候选项}
  F -->|是| D
  F -->|否| X
  E -->|是| G{options.dir 存在}
  G -->|否| H[返回 Ok true]
  G -->|是| I{文件路径以目录开头}
  I -->|是| H
  I -->|否| F
```
