# BurnCloud 架构总览

BurnCloud 是一个 Rust 原生的 LLM 聚合网关，采用四层架构设计。

## 架构分层

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              External Clients                                │
│                    (OpenAI SDK / Claude SDK / Browser / CLI)                │
└─────────────────────────────────────┬───────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Layer 2: Control                                                           │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                        burncloud-server                               │  │
│  │  • REST API (auth, user, channel, token, log, monitor)               │  │
│  │  • LiveView UI (Dioxus)                                              │  │
│  │  • Configuration Management                                          │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                    [详细文档 →](server/README.md)            │
└─────────────────────────────────────┬───────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Layer 1: Gateway (Data Plane)                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                        burncloud-router                               │  │
│  │  • Authentication (Bearer Token)                                     │  │
│  │  • Rate Limiting (Adaptive)                                          │  │
│  │  • Model Routing (via abilities)                                     │  │
│  │  • Protocol Adaptation (OpenAI/Claude/Gemini/Vertex)                 │  │
│  │  • Billing (Tiered/Cache/Batch/Priority)                             │  │
│  │  • Circuit Breaker                                                   │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                    [详细文档 →](router/README.md)            │
└─────────────────────────────────────┬───────────────────────────────────────┘
                                      │
                    ┌─────────────────┼─────────────────┐
                    │                 │                 │
                    ▼                 ▼                 ▼
┌───────────────────────┐ ┌───────────────────┐ ┌───────────────────────┐
│  Layer 3: Service     │ │  Layer 4: Data    │ │  Common               │
│  ┌─────────────────┐  │ │  ┌─────────────┐  │ │  ┌─────────────────┐  │
│  │ service-infer   │  │ │  │ db-router   │  │ │  │ types.rs        │  │
│  │ service-user    │  │ │  │ db-user     │  │ │  │ pricing_config  │  │
│  │ service-models  │  │ │  │ db-models   │  │ │  │ price_u64       │  │
│  │ service-monitor │  │ │  │ db-setting  │  │ │  │ ...             │  │
│  │ ...             │  │ │  │ ...         │  │ │  └─────────────────┘  │
│  └─────────────────┘  │ │  └─────────────┘  │ │                       │
│  [详细→](service/)    │ │  [详细→](database/)│ │  [详细→](common/)     │
└───────────────────────┘ └───────────────────┘ └───────────────────────┘
```

## Crate 索引

| Crate | 层级 | 职责 | 详细文档 |
|-------|------|------|----------|
| `burncloud-router` | Gateway | 数据平面，处理流量、认证、路由 | [→](router/README.md) |
| `burncloud-server` | Control | 控制平面，REST API、LiveView | [→](server/README.md) |
| `burncloud-service` | Service | 业务逻辑层 | [→](service/README.md) |
| `burncloud-database` | Data | 持久化层 | [→](database/README.md) |
| `burncloud-common` | - | 共享类型和工具 | [→](common/README.md) |
| `burncloud-client` | Client | GUI 客户端 (Dioxus) | [→](client/README.md) |
| `burncloud-cli` | CLI | 命令行工具 | [→](cli/README.md) |
| `burncloud-core` | Core | 核心工具和配置 | [→](core/README.md) |

## 数据流

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  Client  │────▶│  Server  │────▶│  Router  │────▶│ Upstream │
│          │     │ (Port    │     │  (Auth,  │     │ Provider │
│ SDK/Browser    │  3000)   │     │  Route)  │     │          │
└──────────┘     └──────────┘     └──────────┘     └──────────┘
                                        │
                      ┌─────────────────┼─────────────────┐
                      │                 │                 │
                      ▼                 ▼                 ▼
                 ┌─────────┐      ┌─────────┐      ┌─────────┐
                 │ Service │      │ Database│      │ Common  │
                 │ (Logic) │      │ (Store) │      │ (Types) │
                 └─────────┘      └─────────┘      └─────────┘
```

## 关键设计原则

1. **Router: "Don't Touch the Body"** - 路由器是智能管道，不解析/修改请求体
2. **零延迟流式传输** - 响应流式透传，无缓冲延迟
3. **双币种钱包** - 支持 USD/CNY 双币种计费
4. **动态协议适配** - 运行时配置的协议转换器

## 快速导航

- [数据流详解](data-flow.md)
- [路由器模块](router/README.md)
- [服务层模块](service/README.md)
- [数据库模式](database/README.md)
