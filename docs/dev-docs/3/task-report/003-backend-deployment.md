# 003 连接后端部署边界：任务报告

## 修改的信息

- 在内部 `ChatBackend` 增加部署边界，默认替身对合法路径返回未连接错误，非法路径返回参数错误。
- `HttpBackend` 委托已有目录客户端部署并保留响应错误阶段；客户端初始化错误归属部署阶段。本地初始化调用路径及签名保持不变。

## 修改的文件与文件内位置

- `crates/interfaces/client/src/domains/chat/backend.rs`：`ChatBackend::deploy_model`、`HttpBackend` 的实现。
- `crates/interfaces/client/src/domains/chat/local_backend_tests.rs`：默认替身、客户端初始化失败及真实本地 HTTP 委托测试。

## 检查结果

- 默认替身返回 `NotConnected`，非法路径返回 `InvalidParameter`；HTTP 委托实测保留健康检查错误及原因。
- 对照 diff 检查 `initialize` 的签名、可见性及本地模型路径未改变；此前本地模型完整回归通过。
- 项目格式、clippy 和测试在提交前后执行；客户端专项测试覆盖上述新增行为。
