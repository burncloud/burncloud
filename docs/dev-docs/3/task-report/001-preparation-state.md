# 001 维护初始化状态及凭据：任务报告

## 修改的信息

- 将 `ArtifactReady` 纳入运行中状态，避免下载校验完成后提前被视为结束。
- 不更改其余状态、类型字段、可见性和凭据所有权；增加独立回归测试。

## 修改的文件与文件内位置

- `crates/interfaces/client/src/domains/chat/model.rs`：`PreparationStage::is_running`。
- `crates/interfaces/client/src/domains/chat/tests.rs`：`artifact_ready_remains_running_without_changing_other_stages`。

## 检查结果

- `ArtifactReady` 与其余各阶段运行状态：回归测试通过。
- `Preparation` 和 `PreparedModel` 模型、版本、摘要及路径字段与可见性保持不变：对照源码和 diff 检查通过。
- `cargo fmt --all -- --check`、`cargo clippy`、`cargo test`：通过；默认 `cargo test` 执行根包 4 项测试。
- `cargo test -p burncloud-client --lib`：添加回归测试前原有 196 项通过；新增回归测试单独执行通过。
