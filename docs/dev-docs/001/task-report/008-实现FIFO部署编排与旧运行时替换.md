# 008 实现 FIFO 部署编排与旧运行时替换

## 修改的信息

- `DeploymentService::new` 启动单个 FIFO worker；服务释放时的 watch 信号会结束 worker 和后台监控，不隐式停止运行时进程。
- 新增 `DeploymentService::deploy`，构造规范队列请求并等待合并后的首次健康结果；worker 只取 `Queued` 事件已成功发布的队首任务，不同模型串行执行。
- 结果交付与移除 `in_flight` 在同一锁内完成，避免结果尚未送达时同模型新请求开启第二个流程；等待者关闭时记录调试日志。
- 新增 `run_deployment`，依次发布 Planning、Starting、WaitingReady、HealthChecking 和首次结果事件，使用 adapter 的唯一计划启动进程，并对同一 readiness target 执行 readiness 与 health。
- 首次 health true/false 分别返回 Ready/Unhealthy 并启动后台监控；请求/计划/启动失败发布 DeploymentFailed，已启动进程失败时先 stop，清理错误附加于主错误。
- 新增 `stop_old_runtime`：先发布 StoppingOldRuntime，再取消并等待旧监控后停止旧进程；stop 失败时重新启动旧监控并保留旧状态。

## 修改的文件与文件内位置

- `crates/platform/node/src/deployment/service.rs`：构造方法 worker 启动、`DeploymentService::deploy`、`run_queue`、`run_deployment`、`stop_old_runtime`、队列取件与事件辅助函数。

## 检查结果

- 不同模型按队列顺序串行；同模型在途请求共享规范结果，完成交付后合并窗口关闭：通过代码检查。
- 旧运行时停止阶段顺序、失败恢复旧监控、计划来源和 readiness/health target 一致：通过代码检查。
- readiness 成功后立即 health；false 保留进程；阶段事件失败会清理已启动新进程并保留 EventPublishFailed 主因：通过代码检查。
- `cargo fmt --all -- --check` 与 `git diff --check`：通过。
- `cargo clippy --offline -p burncloud-node-runtime --all-targets`：未完成；Cargo 离线解析缺少 `aws-lc-rs`，未进入 crate 编译。
