# 002 实现部署 HTTP 适配：任务报告

## 修改的信息

- 按现有部署协议发送一次携带原始路径的 POST，校验完整快照并按部署与健康检查阶段映射结果和错误。
- 按用户确认的方案，关闭共享长请求客户端的自动重试，仅对部署请求禁止跟随重定向；共享客户端上的下载请求也不再自动重试，其他请求保留原有重定向策略。
- 在独立测试文件验证路径、终态、错误、缺失字段、非法数据、超时及重定向。

## 修改的文件与文件内位置

- `crates/interfaces/client/src/domains/chat/catalog.rs`：部署协议类型、`CatalogClient::with_timeouts` 和 `CatalogClient::deploy_model`。
- `crates/interfaces/client/src/domains/chat/catalog_tests.rs`：`deploy_http_*` 独立测试。

## 检查结果

- 原始路径和单次 POST、禁止重定向：本地 HTTP 测试通过；共享客户端配置禁止自动重试，无 GET、stop 或探测实现。
- ready、unhealthy、failed、HTTP 400/503/其他非 200、超时及非法状态：本地 HTTP 测试通过；错误阶段符合设计。
- 全部必需字段缺失、模型 ID 为空、事件非数组、坏 JSON：测试通过；可空字段以 null 测试通过，远端原因及坏错误体降级均已测试。
- 编码检查与测试执行记录见本次阶段汇报；提交后再次运行项目约定检查。
