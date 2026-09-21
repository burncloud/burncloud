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
- 返回值：`Result<(), Box<dyn std::error::Error>>`。成功时创建下载管理器、添加示例下载任务，并每三秒查询状态直至任务完成或出错。
- 错误信息：`DownloadManager::new`、`add_download` 或 `get_status` 返回的错误通过 `?` 转换为 `Box<dyn std::error::Error>` 并向上返回。
