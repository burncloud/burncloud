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
            ├── /api/models/*   → 模型选择（预置 / 流式搜索 / 状态）
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

## 模型选择 v0.2

三个只读接口由 `create_app()` 合并进统一 Router；关闭 LiveView 时仍然可用。它们保持首页免登录目录浏览能力，经过主服务全局请求追踪与安全边界，不启动额外端口或进程。

| GET 接口 | 参数 | 响应 |
| --- | --- | --- |
| `/api/models/presets` | 无 | 18 项数组，保持原顺序；字段为 `id`、`name`、`preset`、`model`、`source`、`revision` |
| `/api/models/search` | 非空 `q` | `application/x-ndjson` 累计快照，每行含 `done`、`items`；仅最终行含 `type=models/unavailable/failed` |
| `/api/models/status` | `preset`，或 `model`、`source`、可选 `revision` | `status`、`model`、`source`、`revision`、`resource_url`；非法资源参数返回 HTTP 400 |

预置标识存在时只使用服务内绑定资源。来源限定为 `modelscope`、`hf-mirror`、`huggingface`；仓库与修订号经过安全校验，只从固定站点构建地址。来源默认修订号分别为 `master`、`main`、`main`。

搜索并发请求两个国内来源，单来源超时 8 秒；只有国内来源超时才向 Hugging Face 回退一次。以小写仓库短名称去重，优先级为 ModelScope > hf-mirror > Hugging Face，高优先级命中原位替换而不打乱到达顺序。至少一个来源成功但没有结果时最终类型为 `unavailable`，全部失败为 `failed`；错误写入主服务日志。断开响应会取消搜索及剩余上游任务。

资源页面通过 GET 探测，超时 10 秒；200 返回 `downloadable`，其余状态或网络失败返回 `developing`。当前没有真实本地状态仓库，不返回伪造的 `available/preparing`，也不提供旧分页、资源二次解析和工件接口。资源页面不是模型文件，“下载”只表示允许把资源快照交给现有初始化入口。

客户端与主服务使用相同的 `HOST`、`PORT`，远程客户端可配置 `BURNCLOUD_SERVER_URL`；LiveView 的 HTTP 请求发生在主服务运行环境。本模块不需要数据库迁移。
