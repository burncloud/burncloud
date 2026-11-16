# 002 下载管理器异步任务生命周期

## 修改的信息

- 为下载管理器新增 `DownloadTaskStatus`、`DownloadTaskHandle` 和 `DownloadTaskSnapshot`，统一承接稳定数据库 ID、当前 aria2 GID、进度、错误和最终路径。
- 将下载流程拆为 `start_download`、`get_task`、`cancel_task` 与 `wait_for_task`：创建接口只登记任务并注册监控，查询按稳定 ID 合并数据库与内存状态，取消按任务锁执行 aria2 / 数据库补偿，等待复用现有 waiter 通知。
- 保留 `DownloadManager::download` 作为兼容包装，所有旧调用方改由异步创建后等待最终路径，未引入第二套等待逻辑。

## 修改的文件与文件内位置

- `crates/platform/storage/download/src/manager.rs`：新增公开任务状态、任务句柄、任务快照，以及任务生命周期锁。
- `crates/platform/storage/download/src/error.rs`：新增任务不存在和任务不可取消错误。
- `crates/platform/storage/download/src/lib.rs`：重新导出异步下载任务公开类型。
- `crates/platform/storage/download/src/operations.rs`：实现 `start_download`、`get_task`、`cancel_task`、`wait_for_task`，重构 `download`、任务登记和 waiter 生命周期。
- `crates/platform/storage/download/tests/public_surface.rs`：覆盖新增管理器入口和公开类型的可见性。

## 检查结果

- `cargo fmt --all -- --check`：通过。
- `cargo clippy -p burncloud-download --all-targets -- -D warnings`：通过。
- `cargo test -p burncloud-download --lib --tests`：通过，16 个测试通过。
- 对照验收标准：任务句柄立即返回；稳定 ID 查询与 aria2 GID 变化解耦；取消在任务锁内处理幂等、终态和补偿；等待复用监控 waiter 并验证最终路径；兼容 `download` 仅组合新入口。
