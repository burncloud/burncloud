# 007 聊天初始化下载状态与取消

## 修改的信息

- 在 `Preparation` 中增加当前 Node 异步下载任务 ID，并增加 `DownloadTaskCreated` 事件，使目录客户端创建任务后立即把稳定 `task_id` 写入当前初始化卡片。
- `ChatState::preparation_event` 对任务 ID 做十进制 `i64` 范围校验，只允许目录模型记录远端任务；重试会清理旧任务 ID，轮询进度、实际摘要、Node 路径和错误仍沿用现有状态字段。
- 增加 `ChatState::request_preparation_cancel`：仅处理匹配的活动运行，原子取出任务 ID并把当前运行置为 `Cancelled`，从而拒绝后续迟到事件；增加取消请求失败转 `Failed` 的状态收敛方法，避免伪装成取消成功。
- 为 `ChatBackend` 增加下载取消抽象：`HttpBackend` 调用 `CatalogClient::cancel_download_task`，默认本地替身返回结构化错误而不伪装远端取消。
- 初始化卡片在 `Downloading` / `Verifying` 阶段提供“取消下载”入口；取消流程先等待 Node `DELETE` 结果，再停止本地 Dioxus 初始化 Future，远端取消失败会保留 `ChatError`。
- 加强取消 HTTP 响应校验：只接受任务 ID 匹配且状态为 `cancelled` 的完整任务快照。

## 修改的文件与文件内位置

- `crates/interfaces/client/src/domains/chat/model.rs`
  - `PreparationEvent`：增加 `DownloadTaskCreated`。
  - `Preparation`：增加 `download_task_id` 字段。
- `crates/interfaces/client/src/domains/chat/state.rs`
  - 初始化与重试流程重置任务 ID。
  - `preparation_event`：记录并校验 Node 任务 ID。
  - `request_preparation_cancel`、`finish_preparation_cancellation`：实现运行失效、任务 ID 取出、取消失败状态收敛和迟到事件隔离。
- `crates/interfaces/client/src/domains/chat/catalog.rs`
  - `download_model`：创建任务成功后回调任务 ID。
  - `cancel_download_task`：提升为聊天模块内部接口，校验返回任务快照并只返回确认的取消结果。
- `crates/interfaces/client/src/domains/chat/backend.rs`
  - `ChatBackend`：增加取消下载方法及本地默认错误语义。
  - `HttpBackend`：接入目录客户端取消请求。
- `crates/interfaces/client/src/domains/chat/page.rs`
  - 初始化卡片增加取消按钮。
  - `cancel_preparation`：实现“先远端 DELETE、后取消本地 Future”的异步顺序。
- `crates/interfaces/client/src/domains/chat/backend_tests.rs`
  - 增加任务 ID 记录、取消后迟到事件隔离和取消失败不伪装成功的状态回归测试。
- `crates/interfaces/client/src/domains/chat/catalog_tests.rs`
  - 更新任务 ID 事件顺序断言。
  - 增加取消快照状态、任务身份匹配和非 `cancelled` 响应测试。

## 检查结果

- `cargo fmt --all -- --check`：通过。
- `cargo clippy -p burncloud-client --all-targets -- -D warnings`：通过。
- `cargo test -p burncloud-client --lib domains::chat -- --nocapture`：通过。
- `cargo test -p burncloud-client --all-targets`：通过，196 个库测试通过，二进制测试 0 个测试通过。
- 对照验收标准：任务 ID 会随创建事件保存；下载和校验阶段可取消；取消先调用服务端 DELETE 后停止本地 Future；取消失败进入 `Failed` 并保存 `ChatError`；运行 ID 不匹配或取消后的迟到事件不会改写当前状态；成功初始化仍可继续既有 `ArtifactReady` / `spawn_stream` 流程。