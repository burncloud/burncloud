# 004 Node 下载任务 HTTP API

## 修改的信息

- 将 Node 模型下载接口从同步等待改为异步任务生命周期：POST 只登记并启动下载，返回 `202 Accepted` 任务快照。
- 注册 `GET /api/node/models/download/{task_id}` 查询接口和 `DELETE /api/node/models/download/{task_id}` 取消接口，统一返回 `Cache-Control: no-store` 的 JSON 响应。
- 在 Node API 模块内增加任务注册表，保存请求身份、校验状态、错误、最终路径和实际 SHA-256；不修改数据库字段。
- 后台校验任务等待 `DownloadManager::wait_for_task`，维护 `verifying`、`succeeded`、`failed` 状态，并在摘要不匹配时返回 `checksum_mismatch`。
- 取消流程使用状态锁与取消标记协调后台摘要校验，避免取消成功后被后台任务覆盖为 `succeeded`；对已取消任务保持幂等，对其他终态返回 `409`。
- 保留原有请求身份、仓库地址、摘要和路径安全校验，任务 ID 仅接受十进制 `i64`。

## 修改的文件与文件内位置

- `crates/interfaces/server/src/api/model_download.rs`
  - `routes`：注册 POST、GET、DELETE 三个下载任务路由。
  - `create_download_task`：请求校验、任务登记、`202` 快照响应和后台摘要校验启动。
  - `get_download_task`：任务 ID 解析、下载管理器快照查询和 `200/400/404/500` 响应。
  - `cancel_download_task`：取消状态协调、`DownloadManager::cancel_task` 调用和 `200/400/404/409/500` 响应。
  - `spawn_task_verification`、`verify_task`：下载等待、SHA-256 校验及 Node 层状态维护。
  - `TaskRegistry`、任务快照转换和错误响应辅助函数：维护进程内任务身份及稳定 HTTP 错误码。
- `crates/interfaces/server/src/api/model_download_tests.rs`
  - 更新成功任务快照响应断言，继续覆盖 JSON 和 `no-store` 响应契约。

## 检查结果

- `cargo fmt --all -- --check`：通过。
- `cargo clippy -p burncloud-server --all-targets -- -D warnings`：通过。
- `cargo test -p burncloud-server --lib --tests`：通过，库测试 77 个、集成测试全部通过（包含 1 个按设计忽略的测试）。
- `git diff --check`：通过。
- 对照验收标准：三条 HTTP 路由已注册；POST 返回异步任务快照；GET、DELETE 错误码和状态码已分流；后台校验与取消并发状态已协调；响应禁止缓存。