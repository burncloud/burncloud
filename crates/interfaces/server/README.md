# burncloud-server

统一网关入口。Axum HTTP 服务,暴露管理 REST API 和 Dioxus LiveView 前端。

## 为什么存在

作为控制平面,Server 层连接所有后端组件。管理 API 通过 Server → Service → Database 的调用链处理 Dashboard 操作。LiveView 托管在同一端口上。

## 关键类型

| 类型 | 说明 |
|------|------|
| `AppState` | Server 运行时状态:db, monitor, force_sync_tx |
| `create_app()` | 构建 Axum Router(合并 API + LiveView + Data Plane) |
| `start_server()` | 启动 HTTP 服务器入口 |

## 依赖

- `burncloud-database` + 子 crate — 数据持久化
- `burncloud-router` — 数据平面(作为 fallback service)
- `burncloud-client` — LiveView 前端
- `burncloud-service-monitor` — 系统监控
- `burncloud-common`, `burncloud-core` — 共享类型

## 架构

```
Request → Axum Router
            ├── /console/api/*  → 管理 API Handler
            ├── /               → LiveView (如果启用)
            └── fallback        → Router (数据平面代理)
```

## API 模块

`crates/interfaces/server/src/api/` 下按认证、管理、监控和硬件能力拆分 HTTP 模块。

- `GET /api/node/hardware`：公开读取运行 Node/Server 主机的一次性硬件快照，不要求 JWT；成功响应禁止缓存。
- 详细请求、响应、错误码和前端接入规则见 [`docs/hardware-api.md`](docs/hardware-api.md)。
- `POST /api/node/models/resolve`：接收前端当前选中的外部仓库模型、来源和修订号，在服务端发现 GGUF 工件、检测硬件并返回全部兼容 Variant；服务端不再内置默认模型 Manifest。
- 硬件检测、模型目录、模型解析和前端调用流程见 [`docs/hardware-model-resolution-api.md`](docs/hardware-model-resolution-api.md)。
- OpenAPI 文档由 `/api-docs/openapi.json` 提供，Swagger UI 入口为 `/swagger-ui`。

## Sources

- `crates/interfaces/server/src/lib.rs` — 应用构建和启动
- `crates/interfaces/server/src/api/mod.rs` — 路由注册
- `docs/backend/api-patterns.md` — 开发指南
