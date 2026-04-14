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

`crates/server/src/api/` 下 7 个模块:auth, channel, group, log, monitor, token, user

## Sources

- `crates/server/src/lib.rs` — 应用构建和启动
- `crates/server/src/api/mod.rs` — 路由注册
- `docs/backend/api-patterns.md` — 开发指南
