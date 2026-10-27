# 009 接入 Node 组合根与应用错误通道

## 修改的信息

- 在 `NodeComposition` 持有 `DeploymentMonitorError` receiver，并提供异步读取方法；通道关闭时返回 `None`。
- 按接口契约扩展 `NodeComposition::start`，接收部署服务创建的 receiver 和共享的适配器 `Arc`；调用方在 Node 组合根生命周期内保留同一个 `DeploymentService`。
- 更新仓库内的既有调用点。未创建部署服务的 fake Node 用例传入已关闭的错误通道，不伪造应用事件发布器。
- 增加生产接线示例：使用一个 `LocalProcessManager::default()` 和同一个 `HttpProbe::default()` 构造部署服务，将共享的适配器和错误 receiver 交给 Node 组合根，并返回组合根与服务以共同持有其生命周期。
- 增加 API 回归测试，验证应用层可读取一条监控错误，且通道关闭后读取返回 `None`。
- 通过 `lib.rs` 导出 `deployment` 模块；其公开项由模块内既有的接口契约导出限定。

## 修改的文件与文件内位置

- `crates/platform/node/src/composition.rs`：`NodeComposition` 错误 receiver 字段、`start`、`next_deployment_monitor_error` 及生产接线文档示例。
- `crates/platform/node/src/lib.rs`：公开导出 `deployment` 模块。
- `crates/platform/node/README.md`：部署监控 receiver 与服务生命周期说明。
- `crates/platform/node/tests/composition.rs`、`crates/platform/node/tests/golden_path.rs`：更新 fake 组合根调用；composition 集成测试覆盖错误读取和通道关闭。
- `crates/interfaces/server/src/node_attachment.rs`、`crates/interfaces/server/src/node_orchestrator.rs`、`crates/interfaces/server/src/node_test.rs`、`crates/interfaces/server/tests/node_artifact_gate_001.rs`、`crates/interfaces/server/tests/node_framework_e2e.rs`、`crates/interfaces/server/tests/node_route_visibility_lifecycle.rs`：更新既有 fake 组合根调用。

## 检查结果

- 组合根持有部署服务创建的 receiver，异步读取返回下一条错误，receiver 关闭时返回 `None`：通过代码检查。
- 生产接线示例在部署服务和组合根间共享同一个进程管理器和同一个 HTTP 探测器：通过代码检查。
- 部署 API 通过 crate 公共模块导出，应用层可读取错误并自行转发：通过代码检查。
- API 回归测试覆盖错误读取和通道关闭返回 `None`；测试因离线依赖解析失败而未执行，见下方命令结果。
- 丢弃组合根会关闭 receiver；部署服务释放时由既有 Drop 实现通知 worker/monitor 退出，不隐式停止运行时进程：通过代码检查。
- `cargo fmt --all -- --check` 与 `git diff --check`：通过。
- `cargo clippy --offline --workspace --all-targets`：未完成；Cargo 离线解析缺少 `aws-lc-rs`，未进入 crate 编译。
- `cargo test --offline --workspace --all-targets`：未完成；同样因缺少 `aws-lc-rs` 在依赖解析阶段退出，未执行测试。
