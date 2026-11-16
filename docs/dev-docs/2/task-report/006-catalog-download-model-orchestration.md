# 006 目录客户端模型下载轮询编排

## 修改的信息

- 将 `CatalogClient::download_model` 从旧的单次下载响应改为异步任务编排：先创建 Node 下载任务，再按固定 200ms 间隔查询任务快照，直到成功、失败或取消终态。
- 创建响应和每次轮询响应均经过任务状态、任务身份、进度、路径、摘要和错误字段校验；网络或超时错误继续映射为客户端连接 / 超时错误，不伪装成服务端任务失败。
- 将 `queued/downloading` 的累计字节转换为 `DownloadProgress`，将 `verifying` 转换为一次 `Downloaded`，成功时校验 Node 路径和实际 SHA-256，并交付 `Verified` 与 `PreparedModel`。
- 将 `failed/cancelled` 终态及非法成功响应映射为 `ChatError`，避免在任务未完成时提前返回成功。
- 删除旧的单次下载响应 DTO 与回调耦合，保留下载取消接口供后续聊天初始化取消流程接入。

## 修改的文件与文件内位置

- `crates/interfaces/client/src/domains/chat/catalog.rs`
  - `CatalogClient::download_model`：实现创建任务、报告初始进度、固定间隔轮询、终态处理和最终摘要校验。
  - `poll_download_task`：支持跨初始快照传递校验阶段事件状态，避免重复发送 `Downloaded`。
  - 删除旧的 `LegacyDownloadEnvelope` / `LegacyDownloadData`，使目录下载客户端只消费异步任务契约。
- `crates/interfaces/client/src/domains/chat/catalog_tests.rs`
  - 增加多响应任务测试服务器和 `queued → downloading → verifying → succeeded` 的轮询编排测试。
  - 更新成功响应、身份不匹配测试以覆盖异步任务快照和校验阶段错误语义。
  - 修正测试 HTTP 响应使用真实 CRLF，验证客户端能够连接并按顺序发送创建及轮询请求。

## 检查结果

- `cargo fmt --all -- --check`：通过。
- `cargo clippy -p burncloud-client --all-targets -- -D warnings`：通过。
- `cargo test -p burncloud-client --lib domains::chat::catalog -- --nocapture`：通过，20 个目录客户端测试通过。
- `cargo test -p burncloud-client --all-targets`：通过，194 个库测试通过，二进制测试 0 个测试通过。
- 对照验收标准：`download_model` 先创建任务并按固定间隔轮询；累计进度、下载完成、摘要校验事件均按状态交付；成功结果包含校验后的 Node 路径和摘要；失败、取消、身份不匹配及非法快照均返回 `ChatError`。