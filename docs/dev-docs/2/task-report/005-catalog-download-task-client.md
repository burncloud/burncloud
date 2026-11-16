# 005 目录客户端下载任务接口

## 修改的信息

- 为 `CatalogClient` 增加下载专用 HTTP 客户端；普通目录请求保留原超时，生产环境 `from_env` 为创建、查询和取消下载任务使用更长的专用超时。
- 增加创建、查询、固定间隔轮询和取消下载任务的 HTTP 方法，统一解析任务信封和服务端错误。
- 增加下载任务快照字段校验：十进制任务 ID、状态、进度范围、模型身份、路径、摘要和失败错误字段均在客户端边界拒绝非法数据。
- 轮询将 `queued/downloading` 转换为累计进度事件，将 `verifying` 转换为下载完成事件，并保留 `succeeded/failed/cancelled` 终态供后续编排处理。
- 取消接口只接受服务端返回的 `cancelled` 快照，不把其他状态伪装成取消成功。

## 修改的文件与文件内位置

- `crates/interfaces/client/src/domains/chat/catalog.rs`
  - `CatalogClient`：增加下载专用 `reqwest::Client` 及生产环境长超时构造。
  - `DownloadTaskData`、`DownloadTaskEnvelope`：增加异步下载任务 DTO。
  - `create_download_task`、`get_download_task`、`poll_download_task`、`cancel_download_task`：实现任务 HTTP 边界。
  - `map_download_task` 及相关校验辅助函数：校验任务快照并映射稳定错误。
- `crates/interfaces/client/src/domains/chat/catalog_tests.rs`
  - 增加任务 ID、身份、进度和终态字段白盒校验测试。

## 检查结果

- `cargo fmt --all`：通过。
- `cargo clippy -p burncloud-client --all-targets -- -D warnings`：通过。
- `cargo test -p burncloud-client --lib domains::chat::catalog -- --nocapture`：通过，19 个测试通过。
- 对照验收标准：创建、查询、轮询、取消接口已形成专用客户端边界；任务快照非法字段会转换为 `ChatError`；轮询采用固定间隔并产生累计进度与校验阶段事件。