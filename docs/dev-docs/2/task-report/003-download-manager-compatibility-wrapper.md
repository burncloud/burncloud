# 003 下载管理器兼容等待包装

## 修改的信息

- 明确现有 `DownloadManager::download` 只承担兼容包装职责：先登记异步任务，再按稳定任务 ID 复用 `wait_for_task` 等待最终路径。
- 未新增第二套等待或通知逻辑；`DownloadError` 由 `start_download` / `wait_for_task` 原样传播。

## 修改的文件与文件内位置

- `crates/platform/storage/download/src/operations.rs`：`DownloadManager::download` 文档与实现，确认兼容入口委托异步生命周期。
- `docs/dev-docs/2/task-report/003-download-manager-compatibility-wrapper.md`：本任务报告。

## 检查结果

- `cargo fmt --all`：通过。
- `cargo clippy -p burncloud-download --all-targets -- -D warnings`：通过。
- `cargo test -p burncloud-download --lib --tests`：通过，16 个测试通过。
- 对照验收标准：兼容入口按 `start_download` → `wait_for_task` 执行；错误不被吞并；模型 HTTP 接口将在后续 Node API 任务中直接使用异步入口。
