# burncloud-service-inference

本地推理服务管理。启动和管理 llama-server 等推理后端进程。

## 关键类型

| 类型 | 说明 |
|------|------|
| `InferenceService` | 推理服务管理(启动/停止/状态查询) |
| `InferenceConfig` | 推理实例配置 |
| `InstanceStatus` | 实例运行状态 |
| `InferenceError` | 推理服务错误 |

## 依赖

- `burncloud-database`, `burncloud-database-router` — 数据持久化
- `burncloud-service-setting` — 配置读取
- `burncloud-service-models` — 模型信息
