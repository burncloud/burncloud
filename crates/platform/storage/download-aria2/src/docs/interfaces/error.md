# error.rs 函数文档

## 公有函数

本文件未直接定义 `pub` 函数或方法。

## 私有函数

### fmt

- 函数定义：
  ```rust
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
  ```
- 入参：
  - `&self`：待格式化的 `Aria2Error`。
  - `f: &mut std::fmt::Formatter<'_>`：标准库格式化输出器。
- 返回值：`std::fmt::Result`。成功时将错误类型及其消息写入格式化输出器。
- 错误信息：返回格式化写入产生的 `std::fmt::Error`；不生成 `Aria2Error`。
