# constants.rs 函数文档

## 公有函数

### get_burncloud_dir

- 函数定义：
  ```rust
  pub(crate) fn get_burncloud_dir() -> PathBuf
  ```
- 入参：无。
- 返回值：`PathBuf`，BurnCloud 本地数据目录。优先使用 `USERPROFILE` 构造路径；读取失败时使用 `C:\Users\Default\AppData\Local\BurnCloud`。
- 错误信息：不返回 `Result`；环境变量读取失败会使用默认路径。
