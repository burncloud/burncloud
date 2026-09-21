# utils.rs 函数文档

## 公有函数

### extract_filename_from_url

- 函数定义：
  ```rust
  pub(crate) fn extract_filename_from_url(url: &str) -> Option<String>
  ```
- 入参：
  - `url: &str`：待解析的资源 URL。
- 返回值：`Option<String>`。URL 路径最后一段非空时返回文件名；URL 不含路径段、查询参数截取前无内容或文件名为空时返回 `None`。
- 错误信息：不返回 `Result`；解析失败通过 `None` 表示。

## 私有函数

无。
