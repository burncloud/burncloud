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

## 本机路由启动清单（Ticket 02）

正常 `start_server` 从 `BURNCLOUD_LOCAL_ROUTE_ATTACHMENTS` 读取 JSON 数组；每项只允许现有 model 和 base_url。PowerShell 示例（仅配置示例，端点必须已真实就绪）：

```powershell
$env:BURNCLOUD_LOCAL_ROUTE_ATTACHMENTS = '[{"model":"qwen-4b","base_url":"http://127.0.0.1:8080"}]'
```

未设置或 [] 正常启动，不创建本机挂载；已设置空字符串、null、非数组、未知/缺失字段、非法模型/地址，以及规范化后重复目标，均启动失败。字段值不做隐式 trim。全部输入经 Router 的同源校验通过后才允许任一探测/挂载。HTTP(S) 地址仅限回环 IP 服务根，不接受域名、路径前缀或凭证。

Server 与 Router 共享同一个 Arc<Database>。管理器首次立即直连 GET /v1/models，精确匹配配置别名，2 秒超时包括响应体，体上限 1 MiB，禁止重定向。失败只影响该项，5 秒后复探；成功后调用一次生产 attach，返回实际存储 ID。attach 错误停止该项自动挂载，不因数据库恢复自动写入重试。

管理器 future 直接由 start_server 持有；服务退出/取消时一起回收，立即停止本机挂载的新派发许可，不主动截断已获许可的请求。重启不自动收养、删除或恢复旧记录，核对事务终局后需显式 detach；本次已经失败停止的启动项不会因清理完成自动再挂。异常提交未知仍需人工核对，未提供跨进程数据库 fencing，也未新增清理 HTTP/CLI。

启动入口在配置校验完成后初始化并发上限与调度策略，再持有 `LocalRouteManager::run`；低层 `create_app` / `create_app_with_node_request_state` 保持原语义，不读取本机清单或派生脱离生命周期的管理任务。运行管理会持续探测、区分临时探测故障与推理故障锁，并在 Server 停止时同步关闭新许可。

`tests/local_route_startup.rs` 通过独立子进程调用真实 start_server，使用独立 SQLite、隔离环境变量和受控 HTTP 模型服务，验证启动/请求/恢复/重启闭环；它不是实际模型加载或推理成功证明。子进程辅助测试有 ignore 标记，但由父测试显式运行，不是被跳过的端到端覆盖。

## 导入自动部署结果（启动时）

已有自动部署输出时，可以改用 `BURNCLOUD_LOCAL_ROUTE_DEPLOYMENTS`，不必手工改写成旧清单。它接受单个标准 JSON 对象或对象数组；最小对象如下，完整输出中的额外部署字段可保留：

```json
{
  "model_id": "local-small-model",
  "status": "ready",
  "state": "Ready",
  "endpoint": "http://127.0.0.1:60544",
  "error": null
}
```

`model_id` 原样映射到模型别名，`endpoint` 映射到服务根地址，仍受上述模型与回环地址合法域约束。字段名和 URL 必须是原始 JSON 内容，不接受 Markdown 转义后的键名或 `[http://...](http://...)` 链接，也不接受 `/v1` 路径前缀。示例端口不表示当前存在可用模型服务。

`status` 必须严格为 `ready`，`state` 必须严格为 `Ready`；`error` 只允许缺省或 null。`stage`、`pid`、`last_heartbeat_unix_ms`、`events` 等部署元数据被忽略，不据此刷新挂载心跳、操作进程或解除故障锁。非 Ready、非 null 错误、缺少必需字段或任一非法/重复目标都会拒绝整份配置，且不会先探测或挂载其中的有效项。`[]` 合法；空字符串、null 或非法 JSON 不合法。

此配置与 `BURNCLOUD_LOCAL_ROUTE_ATTACHMENTS` **互斥**，即使两边均为 `[]` 也拒绝启动。旧变量仍只接受 `model`/`base_url` 数组，不隐式合并。PowerShell 示例（先将实际部署返回值保存为下列 UTF-8 文件，并沿用已有密钥、数据库及计费配置）：

```powershell
if (Test-Path Env:BURNCLOUD_LOCAL_ROUTE_ATTACHMENTS) {
    Remove-Item Env:BURNCLOUD_LOCAL_ROUTE_ATTACHMENTS
}
$env:BURNCLOUD_LOCAL_ROUTE_DEPLOYMENTS = Get-Content -LiteralPath 'E:\project\burncloud-one\deployment-result.json' -Raw -Encoding UTF8
$env:BURNCLOUD_LOCAL_ROUTE_MAX_CONCURRENCY = '1'
cargo run -p burncloud -- server
```

同时确认 `.env` 未设置旧变量，避免 CLI 加载后冲突。本配置仅在新 Server 进程启动时读取，现有服务不会热加载，也没有新增运行中注册 HTTP API。不要让多个 Server 实例共享数据库来绕过重启残留保护；核对旧挂载事务与生命周期后再显式处理残留。

部署 Ready **不等于当前可调度**：管理器仍先直接检查 `/v1/models` 中是否存在完全一致的别名，失败每 5 秒重试，成功后才发布实际挂载 ID。可在日志中查找 `local route startup attachment published`；历史或未来时间戳都不能跳过首检。挂载后请求继续走统一网关 `/v1/chat/completions`，请求 `model` 与 `model_id` 一致，沿用运行心跳、故障锁、并发容量和流式占槽。认证、余额和计费校验不放宽，本机模型不会被自动设为免费。

独立测试覆盖对象/数组导入、无效配置全量拒绝、缺失别名复探后发布、流式满载拒绝及 EOF 释放、同库重启不收养残留；仍只证明受控 HTTP 与 SQLite 闭环，不是样例模型真实推理或 PostgreSQL 验收。

## Ticket 03–06 运行管理与调度

`BURNCLOUD_LOCAL_ROUTE_MAX_CONCURRENCY` 未配置时为 `1`；必须是 `1..=u32::MAX` 的十进制值，非法值在任何模型探测前拒绝启动。挂载槽位按实际 ID 独立计数，流式响应在完整消费、错误或取消前保持占用。

`SCHEDULER_POLICIES` 未配置或未指定用户组时使用 `balanced`。新策略在最高可用优先级档内按正权重、最高有限运行健康分、最低严格可比较参考价选择；旧 `passthrough` 与 `combined` 显式配置保持兼容。非法策略、重复大小写组名、字段错配或非法权重拒绝启动。

本轮只以受控 HTTP、SQLite 和现有测试资源验收；真实模型推理与独立 PostgreSQL 运行环境未作为本轮通过条件。
