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
| `ProductionNode` | 生产硬件探测与模型解析入口，由 `AppState.production_node` 持有 |

## 真实硬件与模型解析

服务组装时创建 `ProductionNode`。它使用 `SystemHardwareProbe` 和
`RealModelResolver`，在收到所选模型后执行真实探测，将完整 `HardwareProfile`
交给 resolver 的 RAM/VRAM 判断。它不依赖 `BURNCLOUD_NODE_TEST_API`。

```rust,no_run
use burncloud_server::AppState;
use burncloud_supply_model::{ModelManifest, ModelResolutionOutcome};

async fn submit_selected_model(
    state: &AppState,
    model: &str,
    manifests: Vec<ModelManifest>,
) -> anyhow::Result<ModelResolutionOutcome> {
    state.production_node.resolve_selected_model(model, manifests).await
}
```

调用方提供已加载的模型清单。空模型名、探测失败或模型不存在返回错误，
没有兼容变体返回 `Unsupported`。本入口只做探测与解析，不启动下载或运行时。
上游“模型选择”接口尚未确定，因此目前未新增 HTTP 路由或接入具体页面；
该接口确定后应调用上述入口，完整服务触发链路仍需届时验收。

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

`crates/interfaces/server/src/api/` 下 7 个模块:auth, channel, group, log, monitor, token, user

## Sources

- `crates/interfaces/server/src/lib.rs` — 应用构建和启动
- `crates/interfaces/server/src/api/mod.rs` — 路由注册
- `docs/backend/api-patterns.md` — 开发指南
