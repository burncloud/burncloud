# main.rs 函数文档

## 公有函数

无。

## 私有函数

### main

- 函数定义：
  ```rust
  async fn main() -> Result<(), Box<dyn std::error::Error>>
  ```
- 入参：无。
- 返回值：`Result<(), Box<dyn std::error::Error>>`；成功时完成管理器初始化、示例任务创建及状态轮询。
- 错误信息：管理器创建、添加下载或状态查询失败时，通过 `?` 转换并返回 `Box<dyn std::error::Error>`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[创建 DownloadManager]
  B --> C{创建成功}
  C -->|否| X[返回错误]
  C -->|是| D[添加示例下载任务]
  D --> E{添加成功}
  E -->|否| X
  E -->|是| F[每 3 秒查询任务状态]
  F --> G{状态查询成功}
  G -->|否| X
  G -->|是| H{状态为 complete 或 error}
  H -->|否| I[输出状态并等待 3 秒]
  I --> F
  H -->|是| J[输出完成或失败信息]
  J --> K[返回 Ok]
```
