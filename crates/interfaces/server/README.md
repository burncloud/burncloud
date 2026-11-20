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

`crates/interfaces/server/src/api/` 下 7 个模块:auth, channel, group, log, monitor, token, user

### 模型清单

`GET /api/models/presets` 返回 `src/model_selection/presets.json` 中的 388 条记录，
来源为 2026-11-20 提供的 `Ollama开源模型硬件信息(1).xlsx` 的“模型信息”工作表。
名称、顺序、模型链接及六项配置按原表保留；重复名称或链接不合并。硬件容量字段返回时统一保留两位小数，
例如 `6.00G`。

每项包含 `id`、`name`、`preset`、`model`、`source`、`revision`，以及：

| 字段 | 说明 |
|------|------|
| `resource_url` | 原表模型链接，供客户端直接打开 |
| `hardware.vram_min` | 最低显存，保留原表单位并格式化为两位小数 |
| `hardware.ram_min` | 最低内存，保留原表单位并格式化为两位小数 |
| `hardware.disk_min` | 最低磁盘空间，保留原表单位并格式化为两位小数 |
| `hardware.vram_recommended` | 推荐显存，格式化为两位小数 |
| `hardware.ram_recommended` | 推荐内存，格式化为两位小数 |
| `hardware.disk_recommended` | 推荐磁盘空间，格式化为两位小数 |

`id` 使用小写仓库短名；重复时追加 `-row-<原表行号>`，保证每条记录都可独立选择。
ModelScope 绑定 `master`，HF Mirror 绑定 `main`。`GET /api/models/status?preset=<id>`
仍按该项绑定的仓库查询状态。表内链接与配置是目录资料，不是经过摘要校验的模型工件；
下载和部署继续使用现有 Manifest 的文件地址、摘要和量化版本要求。

## Sources

- `crates/interfaces/server/src/lib.rs` — 应用构建和启动
- `crates/interfaces/server/src/api/mod.rs` — 路由注册
- `docs/backend/api-patterns.md` — 开发指南
