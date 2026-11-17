# 006 协调页面任务与资源生命周期：任务报告

## 修改的信息

- 目录下载成功后同步认领，再委托后端部署并按结果完成状态；非目录模型保留原初始化路径。
- 初始化任务句柄关联 run_id，取消和结束时仅清理匹配句柄；仅未完成下载发远端取消，部署取消只释放本地任务。
- 增加工件就绪和部署期间的取消入口，不发部署 stop；不跨网络等待持有状态 Signal 借用。

## 修改的文件与文件内位置

- `crates/interfaces/client/src/domains/chat/page.rs`：`PreparationTask`、`ChatHome` 清理 effect、取消按钮、`cancel_preparation`、`spawn_preparation`。
- `crates/interfaces/client/src/domains/chat/ui_tests.rs`：`DeploymentBackend` 及目录模型部署成功、部署取消的组件测试。

## 检查结果

- 模拟目录下载后部署一次、成功显示就绪；部署期间取消释放本地 future，不发下载取消：组件测试通过。
- 句柄读写按 run_id 保护，异步调用前释放 Signal 借用：对照源码检查通过；更广泛的竞态覆盖留给任务 007。
- 原有非目录模型与下载取消回归在完整客户端测试中检查；格式、clippy 和测试按项目约定提交前后执行。
