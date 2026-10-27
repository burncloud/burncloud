# 005 实现部署服务与 FIFO 请求协调

## 修改的信息

- 新增 `DeploymentService` 共享内层状态，保存进程/readiness/health/事件接口、监控错误通道、协调器锁、队列通知与关闭信号；服务释放时发送关闭通知，不停止运行时进程。
- 新增 FIFO 队列、同模型在途等待者和受监控运行时数据结构。队列项按入口加锁顺序入队；重复 `model_id` 请求加入同一等待者集合。
- 新增 `publish_with_retry`，对同一个事件最多顺序发布两次。
- 新增 `enqueue_or_join`，先登记在途项并入队，再在锁外发布 `Queued`。发布成功后标记该项可执行并通知 worker；发布失败时移除队列项并将同一错误交付给全部等待者，同时保留当前旧运行时状态。
- 新增 `complete_waiters`，向所有等待者发送同一克隆结果；接收端关闭时记录调试日志。
- 队列 worker 字段和通知状态已预留；worker 创建、消费与部署状态机由任务 008 接入。本任务提供队列状态和入口协调基础。

## 修改的文件与文件内位置

- `crates/platform/node/src/deployment/service.rs`：服务、协调器及队列数据结构，构造方法，事件重试与入队/合并逻辑，等待者交付及关闭通知。
- `crates/platform/node/src/deployment/mod.rs`：声明 `service` 模块并导出 `DeploymentService`；按格式化要求整理模块顺序。
- `crates/platform/node/src/deployment/http_probe.rs`：整理 crate 导入格式，无行为变化。

## 检查结果

- 同模型在途请求合并、不同请求按入队顺序保存，且队列事件失败会清理在途项并共享错误：通过代码检查。
- 发布事件时不持有 coordinator 锁；服务关闭会发出 watch 通知，不停止共享进程：通过代码检查。
- 队列 worker 尚未创建或消费请求，将由任务 008 接入；此处已建立 worker 句柄字段、关闭和通知状态。
- `cargo fmt --all -- --check` 与 `git diff --check`：通过。
- Clippy/编译：受离线缓存缺少 `aws-lc-rs` 阻塞，尚无法检查 Rust 类型与 lint。
