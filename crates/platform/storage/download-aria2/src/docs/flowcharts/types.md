# types.rs

## 公有函数

### is_running
- 函数定义：`pub fn is_running(&mut self) -> bool`
- 入参：`&mut self`；包含 aria2 子进程的运行实例。
- 返回值：`bool`；子进程未退出时为 `true`，已退出或查询失败时为 `false`。
- 错误信息：不返回错误；`try_wait` 失败转换为 `false`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[调用 process.try_wait]
  B --> C{结果为 Ok None}
  C -->|是| D[返回 true]
  C -->|否| E[返回 false]
```

### kill
- 函数定义：`pub fn kill(&mut self) -> Aria2Result<()>`
- 入参：`&mut self`；包含 aria2 子进程的运行实例。
- 返回值：`Aria2Result<()>`；成功时子进程已被终止并完成等待。
- 错误信息：`kill` 或 `wait` 失败时返回 `Aria2Error::ProcessError`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[调用 process.kill]
  B --> C{终止成功}
  C -->|否| D[返回 ProcessError]
  C -->|是| E[调用 process.wait]
  E --> F{等待成功}
  F -->|否| D
  F -->|是| G[返回 Ok]
```

## 私有函数

### default
- 函数定义：`fn default() -> Self`
- 入参：无。
- 返回值：`Self`；包含默认端口、下载目录、连接配置和 aria2 路径的 `Aria2Config`。
- 错误信息：不返回错误；当前目录读取失败时以空路径拼接 `downloads`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[读取当前工作目录]
  B --> C{读取成功}
  C -->|是| D[拼接 downloads]
  C -->|否| E[使用空路径并拼接 downloads]
  D --> F[组装默认 Aria2Config]
  E --> F
  F --> G[返回配置]
```
