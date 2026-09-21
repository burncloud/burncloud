# utils.rs 函数文档

## 公有函数

### extract_filename_from_url

- 函数定义：
  ```rust
  pub(crate) fn extract_filename_from_url(url: &str) -> Option<String>
  ```
- 入参：`url: &str`，待解析的资源 URL。
- 返回值：`Option<String>`；路径最后一段非空时返回文件名，否则返回 `None`。
- 错误信息：不返回 `Result`；URL 解析失败或文件名为空时返回 `None`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[按 ? 分割 URL]
  B --> C{存在主 URL 部分}
  C -->|否| D[返回 None]
  C -->|是| E[按 / 分割并取得最后一段]
  E --> F{取得路径段成功}
  F -->|否| D
  F -->|是| G{文件名为空}
  G -->|是| D
  G -->|否| H[转换为 String]
  H --> I[返回 Some 文件名]
```

## 私有函数

无。
